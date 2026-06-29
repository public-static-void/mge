//! Faction and Reputation API: set_faction, get_faction, modify_reputation, get_reputation.

use engine_core::ecs::world::World;
use engine_core::faction;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the faction/reputation API.
pub fn register_faction_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // set_faction(entity, faction_id, role)
    let w = world.clone();
    let set_faction_fn = lua.create_function_mut(
        move |_, (entity, faction_id, role): (u32, String, String)| {
            let mut world = w.borrow_mut();
            faction::set_faction(&mut world, entity, &faction_id, &role)
                .map_err(mlua::Error::external)?;
            Ok(())
        },
    )?;
    globals.set("set_faction", set_faction_fn)?;

    // get_faction(entity) -> string | nil
    let w = world.clone();
    let get_faction_fn = lua.create_function_mut(move |_, entity: u32| {
        let world = w.borrow();
        Ok(faction::get_faction(&world, entity))
    })?;
    globals.set("get_faction", get_faction_fn)?;

    // modify_reputation(entity, faction_id, delta)
    let w = world.clone();
    let modify_reputation_fn =
        lua.create_function_mut(move |_, (entity, faction_id, delta): (u32, String, i64)| {
            let mut world = w.borrow_mut();
            faction::modify_reputation(&mut world, entity, &faction_id, delta)
                .map_err(mlua::Error::external)?;
            Ok(())
        })?;
    globals.set("modify_reputation", modify_reputation_fn)?;

    // get_reputation(entity, faction_id) -> i64
    let w = world;
    let get_reputation_fn =
        lua.create_function_mut(move |_, (entity, faction_id): (u32, String)| {
            let world = w.borrow();
            Ok(faction::get_reputation(&world, entity, &faction_id))
        })?;
    globals.set("get_reputation", get_reputation_fn)?;

    Ok(())
}
