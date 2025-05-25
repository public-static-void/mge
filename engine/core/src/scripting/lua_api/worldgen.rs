//! Worldgen plugin API for Lua.

use crate::scripting::helpers::{json_to_lua_table, lua_table_to_json};
use crate::worldgen::WorldgenRegistry;
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

    let registry_for_invoke: Rc<RefCell<WorldgenRegistry>> = Rc::clone(&worldgen_registry);
    let invoke_worldgen = lua.create_function(move |lua, (name, params): (String, Table)| {
        let params_json: JsonValue = lua_table_to_json(lua, &params, None)?;
        let result = registry_for_invoke
            .borrow()
            .invoke(&name, &params_json)
            .map_err(|e| mlua::Error::external(format!("{:?}", e)))?;
        json_to_lua_table(lua, &result)
    })?;
    globals.set("invoke_worldgen", invoke_worldgen)?;

    Ok(())
}
