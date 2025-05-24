use crate::scripting::helpers::{json_to_lua_table, lua_error_msg, lua_table_to_json};
use crate::worldgen::{WorldgenError, WorldgenPlugin, WorldgenRegistry};
use mlua::{Function, Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_worldgen_functions(
    lua: &Lua,
    globals: &Table,
    worldgen_registry: Rc<RefCell<WorldgenRegistry>>,
) -> LuaResult<()> {
    // register_worldgen(name, func)
    let worldgen_registry_register = worldgen_registry.clone();
    let register_worldgen = lua.create_function(move |lua, (name, func): (String, Function)| {
        let func_registry_key = lua.create_registry_value(func)?;
        worldgen_registry_register
            .borrow_mut()
            .register(WorldgenPlugin::Lua {
                name,
                registry_key: func_registry_key,
            });
        Ok(())
    })?;
    globals.set("register_worldgen", register_worldgen)?;

    // list_worldgen()
    let worldgen_registry_list = worldgen_registry.clone();
    let list_worldgen =
        lua.create_function(move |_, ()| Ok(worldgen_registry_list.borrow().list_names()))?;
    globals.set("list_worldgen", list_worldgen)?;

    // invoke_worldgen(name, params)
    let worldgen_registry_invoke = worldgen_registry.clone();
    let invoke_worldgen = lua.create_function(move |lua, (name, params): (String, Table)| {
        let params_json = lua_table_to_json(lua, &params, None)?;
        let registry = worldgen_registry_invoke.borrow();
        match registry.invoke_lua(lua, &name, &params_json) {
            Ok(result_json) => json_to_lua_table(lua, &result_json),
            Err(WorldgenError::NotFound) => Err(lua_error_msg(
                lua,
                &format!("Worldgen plugin '{}' not found", name),
            )),
            Err(WorldgenError::LuaError(e)) => Err(e),
        }
    })?;
    globals.set("invoke_worldgen", invoke_worldgen)?;

    Ok(())
}
