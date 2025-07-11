//! Turn API: tick simulation, get current turn.

use crate::lua_api::job_system::process_lua_job_calls;
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers turn-related Lua API functions.
pub fn register_turn_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // tick()
    let world_tick = world.clone();
    let lua_ref = lua.clone();
    let world_ref = world.clone();
    let tick = lua.create_function_mut(move |_, ()| {
        World::tick(Rc::clone(&world_tick));
        process_lua_job_calls(&lua_ref, &world_ref);
        Ok(())
    })?;
    globals.set("tick", tick)?;

    // get_turn()
    let world_get_turn = world.clone();
    let get_turn = lua.create_function_mut(move |_, ()| {
        let world = world_get_turn.borrow();
        Ok(world.turn)
    })?;
    globals.set("get_turn", get_turn)?;

    Ok(())
}
