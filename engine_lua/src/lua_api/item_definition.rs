//! Item definition scripting API for Lua.
//!
//! Provides functions to load, register, query, and list item definitions
//! backed by the engine's `ItemRegistry`.

use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers item definition API functions into the Lua globals.
pub fn register_item_definition_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // load_item_definitions(dir: string)
    let w = world.clone();
    let load_item_definitions = lua.create_function_mut(move |_, dir: String| {
        let path = std::path::Path::new(&dir);
        let mut world = w.borrow_mut();
        world
            .load_item_definitions(path)
            .map_err(mlua::Error::runtime)
    })?;
    globals.set("load_item_definitions", load_item_definitions)?;

    // register_item(item_json: string)
    let w = world.clone();
    let register_item = lua.create_function_mut(move |_, item_json: String| {
        let definition: serde_json::Value = serde_json::from_str(&item_json)
            .map_err(|e| mlua::Error::runtime(format!("Invalid item JSON: {e}")))?;
        let mut world = w.borrow_mut();
        world
            .item_registry
            .register_item(definition)
            .map_err(mlua::Error::runtime)
    })?;
    globals.set("register_item", register_item)?;

    // get_item_definition(id: string) -> table | nil
    let w = world.clone();
    let get_item_definition = lua.create_function_mut(move |lua, id: String| {
        let world = w.borrow();
        match world.get_item_definition(&id) {
            Some(def) => {
                let json =
                    serde_json::to_value(def).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                crate::helpers::json_to_lua_table(lua, &json).map(Some)
            }
            None => Ok(None),
        }
    })?;
    globals.set("get_item_definition", get_item_definition)?;

    // list_item_definitions() -> table (array of strings)
    let w = world.clone();
    let list_item_definitions = lua.create_function_mut(move |_, ()| {
        let world = w.borrow();
        let ids = world.item_registry.list_items();
        Ok(ids)
    })?;
    globals.set("list_item_definitions", list_item_definitions)?;

    Ok(())
}
