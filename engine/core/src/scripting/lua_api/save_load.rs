//! Save/Load API: world serialization.

use crate::ecs::world::World;
use crate::scripting::helpers::lua_error_from_any;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_save_load_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // save_to_file(filename)
    let world_save = world.clone();
    let save_to_file = lua.create_function_mut(move |lua, filename: String| {
        let world = world_save.borrow();
        world
            .save_to_file(std::path::Path::new(&filename))
            .map_err(|e| lua_error_from_any(lua, e))
    })?;
    globals.set("save_to_file", save_to_file)?;

    // load_from_file(filename)
    let world_load = world.clone();
    let registry = world.borrow().registry.clone();
    let load_from_file = lua.create_function_mut(move |lua, filename: String| {
        let mut world = world_load.borrow_mut();
        let loaded = World::load_from_file(std::path::Path::new(&filename), registry.clone())
            .map_err(|e| lua_error_from_any(lua, e))?;
        *world = loaded;
        Ok(())
    })?;
    globals.set("load_from_file", load_from_file)?;

    Ok(())
}
