//! Mode API: get/set mode, list available modes.

use crate::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers mode-related Lua API functions.
pub fn register_mode_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // set_mode(mode: String)
    let world_set_mode = world.clone();
    let set_mode = lua.create_function_mut(move |_, mode: String| {
        let mut world = world_set_mode.borrow_mut();
        world.current_mode = mode;
        Ok(())
    })?;
    globals.set("set_mode", set_mode)?;

    // get_mode()
    let world_get_mode = world.clone();
    let get_mode = lua.create_function_mut(move |_, ()| {
        let world = world_get_mode.borrow();
        Ok(world.current_mode.clone())
    })?;
    globals.set("get_mode", get_mode)?;

    // get_available_modes()
    let world_get_modes = world.clone();
    let get_available_modes = lua.create_function_mut(move |_, ()| {
        let world = world_get_modes.borrow();
        let modes = world.registry.lock().unwrap().all_modes();
        Ok(modes.into_iter().collect::<Vec<String>>())
    })?;
    globals.set("get_available_modes", get_available_modes)?;

    Ok(())
}
