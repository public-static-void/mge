use crate::helpers::{json_to_lua_table, lua_table_to_json, lua_value_to_json};
use engine_core::worldgen::{ScriptingWorldgenPlugin, WorldgenPlugin, WorldgenRegistry};
use mlua::{Function, Lua, RegistryKey, Result as LuaResult, Table};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

struct LuaWorldgenPlugin {
    lua: Rc<Lua>,
    func_key: RegistryKey,
}

impl ScriptingWorldgenPlugin for LuaWorldgenPlugin {
    fn invoke(&self, params: &JsonValue) -> Result<JsonValue, Box<dyn std::error::Error>> {
        let func: Function = self.lua.registry_value(&self.func_key)?;
        let params_table = json_to_lua_table(&self.lua, params)?;
        let result: mlua::Value = func.call(params_table)?;
        lua_value_to_json(&self.lua, result, None).map_err(|e| Box::new(e) as _)
    }
    fn backend(&self) -> &str {
        "lua"
    }
}

pub fn register_worldgen_api(
    lua: &Lua,
    globals: &Table,
    worldgen_registry: Rc<RefCell<WorldgenRegistry>>,
) -> LuaResult<()> {
    let registry_for_list = Rc::clone(&worldgen_registry);
    let list_plugins = lua.create_function(move |_, ()| {
        let plugins = registry_for_list.borrow().list_names();
        Ok(plugins)
    })?;
    globals.set("list_worldgen_plugins", list_plugins)?;

    let registry_for_invoke = Rc::clone(&worldgen_registry);
    let lua_rc = Rc::new(lua.clone());
    let invoke_worldgen_plugin = {
        let lua_rc = Rc::clone(&lua_rc);
        lua.create_function(move |_, (name, params): (String, Table)| {
            let params_json: JsonValue = lua_table_to_json(&lua_rc, &params, None)?;
            let result = registry_for_invoke
                .borrow()
                .invoke(&name, &params_json)
                .map_err(|e| mlua::Error::external(format!("{:?}", e)))?;
            json_to_lua_table(&lua_rc, &result)
        })?
    };
    globals.set("invoke_worldgen_plugin", invoke_worldgen_plugin)?;

    // Registers a Lua worldgen plugin
    let worldgen_registry_for_register = Rc::clone(&worldgen_registry);
    let register_worldgen_plugin = {
        let lua_rc = Rc::clone(&lua_rc);
        lua.create_function(move |lua, (name, func): (String, Function)| {
            let func_key = lua.create_registry_value(func)?;
            let plugin = LuaWorldgenPlugin {
                lua: Rc::clone(&lua_rc),
                func_key,
            };
            worldgen_registry_for_register
                .borrow_mut()
                .register(WorldgenPlugin::Scripting {
                    name,
                    backend: "lua".to_string(),
                    opaque: Box::new(plugin),
                });
            Ok(())
        })?
    };
    globals.set("register_worldgen_plugin", register_worldgen_plugin)?;

    Ok(())
}
