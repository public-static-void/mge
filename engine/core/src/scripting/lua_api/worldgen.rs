//! Worldgen plugin API for Lua.

use crate::scripting::helpers::{json_to_lua_table, lua_table_to_json};
use crate::worldgen::{WorldgenPlugin, WorldgenRegistry};
use mlua::{Lua, Result as LuaResult, Table};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_worldgen_api(
    lua: &Lua,
    globals: &Table,
    worldgen_registry: Rc<RefCell<WorldgenRegistry>>,
) -> LuaResult<()> {
    let registry_for_list: Rc<RefCell<WorldgenRegistry>> = Rc::clone(&worldgen_registry);
    let list_plugins = lua.create_function(move |_, ()| {
        let plugins = registry_for_list.borrow().list_names();
        Ok(plugins)
    })?;
    globals.set("list_worldgen_plugins", list_plugins)?;

    let registry_for_invoke = Rc::clone(&worldgen_registry);
    let invoke_worldgen_plugin =
        lua.create_function(move |lua, (name, params): (String, Table)| {
            let params_json: JsonValue = lua_table_to_json(lua, &params, None)?;
            // Try Lua plugins first
            if let Ok(result) = registry_for_invoke
                .borrow()
                .invoke_lua(lua, &name, &params_json)
            {
                return json_to_lua_table(lua, &result);
            }
            // Fallback to native plugins
            let result = registry_for_invoke
                .borrow()
                .invoke(&name, &params_json)
                .map_err(|e| mlua::Error::external(format!("{:?}", e)))?;
            json_to_lua_table(lua, &result)
        })?;
    globals.set("invoke_worldgen_plugin", invoke_worldgen_plugin)?;

    // Registers a Lua worldgen plugin
    let worldgen_registry_for_register = Rc::clone(&worldgen_registry);
    let register_worldgen_plugin =
        lua.create_function(move |lua, (name, func): (String, mlua::Function)| {
            let registry_key = lua.create_registry_value(func)?;
            worldgen_registry_for_register
                .borrow_mut()
                .register(WorldgenPlugin::Lua { name, registry_key });
            Ok(())
        })?;
    globals.set("register_worldgen_plugin", register_worldgen_plugin)?;

    Ok(())
}
