//! Diplomacy API: get_relation, get_standing, modify_standing, declare_war,
//! declare_peace, propose_treaty, accept_treaty, break_treaty, list_treaties.
//!
//! Thin delegation over [`engine_core::diplomacy`]: relation bounds and treaty
//! rules live in core, so every bridge observes identical behavior.

use crate::helpers::json_to_lua_table;
use engine_core::diplomacy::{self, TreatyKind};
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the diplomacy API.
pub fn register_diplomacy_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_relation(fa, fb) -> "allied" | "neutral" | "hostile" | "war"
    let w = world.clone();
    let get_relation_fn =
        lua.create_function_mut(move |_, (fa, fb): (String, String)| -> LuaResult<String> {
            let world = w.borrow();
            Ok(diplomacy::get_relation(&world, &fa, &fb)
                .as_str()
                .to_string())
        })?;
    globals.set("get_relation", get_relation_fn)?;

    // get_standing(fa, fb) -> integer
    let w = world.clone();
    let get_standing_fn =
        lua.create_function_mut(move |_, (fa, fb): (String, String)| -> LuaResult<i64> {
            let world = w.borrow();
            Ok(diplomacy::get_standing(&world, &fa, &fb))
        })?;
    globals.set("get_standing", get_standing_fn)?;

    // modify_standing(fa, fb, delta) — errors on invalid pairs
    let w = world.clone();
    let modify_standing_fn = lua.create_function_mut(
        move |_, (fa, fb, delta): (String, String, i64)| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            diplomacy::modify_standing(&mut world, &fa, &fb, delta)
                .map_err(mlua::Error::external)?;
            Ok(())
        },
    )?;
    globals.set("modify_standing", modify_standing_fn)?;

    // declare_war(fa, fb) — errors when the pair is already at war
    let w = world.clone();
    let declare_war_fn =
        lua.create_function_mut(move |_, (fa, fb): (String, String)| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            diplomacy::declare_war(&mut world, &fa, &fb).map_err(mlua::Error::external)?;
            Ok(())
        })?;
    globals.set("declare_war", declare_war_fn)?;

    // declare_peace(fa, fb) — errors when the pair is not at war
    let w = world.clone();
    let declare_peace_fn =
        lua.create_function_mut(move |_, (fa, fb): (String, String)| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            diplomacy::declare_peace(&mut world, &fa, &fb).map_err(mlua::Error::external)?;
            Ok(())
        })?;
    globals.set("declare_peace", declare_peace_fn)?;

    // propose_treaty(proposer, other, kind, duration_ticks?) -> treaty id
    // kind: "non_aggression" | "alliance" | "peace" | "trade"
    let w = world.clone();
    let propose_treaty_fn = lua.create_function_mut(
        move |_, args: (String, String, String, Option<u64>)| -> LuaResult<i64> {
            let (proposer, other, kind, duration) = args;
            let kind = TreatyKind::parse(&kind).map_err(mlua::Error::external)?;
            let mut world = w.borrow_mut();
            let id = diplomacy::propose_treaty(&mut world, &proposer, &other, kind, duration)
                .map_err(mlua::Error::external)?;
            Ok(id as i64)
        },
    )?;
    globals.set("propose_treaty", propose_treaty_fn)?;

    // accept_treaty(id) — errors unless the treaty is proposed
    let w = world.clone();
    let accept_treaty_fn = lua.create_function_mut(move |_, treaty_id: i64| -> LuaResult<()> {
        let mut world = w.borrow_mut();
        diplomacy::accept_treaty(&mut world, treaty_id as u64).map_err(mlua::Error::external)?;
        Ok(())
    })?;
    globals.set("accept_treaty", accept_treaty_fn)?;

    // break_treaty(id) — errors unless the treaty is still live
    let w = world.clone();
    let break_treaty_fn = lua.create_function_mut(move |_, treaty_id: i64| -> LuaResult<()> {
        let mut world = w.borrow_mut();
        diplomacy::break_treaty(&mut world, treaty_id as u64).map_err(mlua::Error::external)?;
        Ok(())
    })?;
    globals.set("break_treaty", break_treaty_fn)?;

    // list_treaties(faction?) -> array of treaty records
    let w = world;
    let list_treaties_fn =
        lua.create_function_mut(move |lua, faction: Option<String>| -> LuaResult<LuaValue> {
            let world = w.borrow();
            let records: Vec<serde_json::Value> =
                diplomacy::list_treaties(&world, faction.as_deref())
                    .iter()
                    .map(|treaty| treaty.to_json())
                    .collect();
            json_to_lua_table(lua, &serde_json::Value::Array(records))
        })?;
    globals.set("list_treaties", list_treaties_fn)?;

    Ok(())
}
