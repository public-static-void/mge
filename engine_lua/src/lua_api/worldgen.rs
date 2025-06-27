use crate::helpers::{json_to_lua_table, lua_table_to_json, lua_value_to_json};
use engine_core::worldgen::{ScriptingWorldgenPlugin, WorldgenPlugin, WorldgenRegistry};
use mlua::{Function, Lua, RegistryKey, Result as LuaResult, Table, Value as LuaValue};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

struct LuaWorldgenPlugin {
    lua: Rc<Lua>,
    func_key: RegistryKey,
}

impl Clone for LuaWorldgenPlugin {
    fn clone(&self) -> Self {
        // SAFETY: Rc<Lua> is shared and valid; registry_value gives us a Function,
        // create_registry_value stores a new key for the same function.
        let func: Function = self
            .lua
            .registry_value(&self.func_key)
            .expect("registry_value failed");
        let new_key = self
            .lua
            .create_registry_value(func)
            .expect("create_registry_value failed");
        LuaWorldgenPlugin {
            lua: Rc::clone(&self.lua),
            func_key: new_key,
        }
    }
}

impl ScriptingWorldgenPlugin for LuaWorldgenPlugin {
    fn invoke(&self, params: &JsonValue) -> Result<JsonValue, Box<dyn std::error::Error>> {
        let func: Function = self.lua.registry_value(&self.func_key)?;
        let params_table = json_to_lua_table(&self.lua, params)?;
        let result: LuaValue = func.call(params_table)?;
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
                .map_err(|e| mlua::Error::external(format!("{e:?}")))?;
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

    // Register a validator from Lua (not Send+Sync)
    let worldgen_registry_for_validator = Rc::clone(&worldgen_registry);
    let lua_rc_outer = Rc::clone(&lua_rc);
    let register_validator = lua.create_function(move |lua, func: Function| {
        let func_key = lua.create_registry_value(func)?;
        let lua_rc_inner = Rc::clone(&lua_rc_outer);
        worldgen_registry_for_validator
            .borrow_mut()
            .register_scripting_validator(move |map| {
                let lua_rc = Rc::clone(&lua_rc_inner);
                let func: Function = lua_rc.registry_value(&func_key).unwrap();
                let map_tbl = json_to_lua_table(&lua_rc, map).unwrap();
                let result: LuaValue = func.call(map_tbl).unwrap();
                match result {
                    LuaValue::Nil | LuaValue::Boolean(true) => Ok(()),
                    LuaValue::String(s) => Err(s.to_str().unwrap().to_string()),
                    _ => Err("Validator failed".to_string()),
                }
            });
        Ok(())
    })?;
    globals.set("register_worldgen_validator", register_validator)?;

    // Register a postprocessor from Lua (not Send+Sync)
    let worldgen_registry_for_post = Rc::clone(&worldgen_registry);
    let lua_rc_outer = Rc::clone(&lua_rc);
    let register_postprocessor = lua.create_function(move |lua, func: Function| {
        let func_key = lua.create_registry_value(func)?;
        let lua_rc_inner = Rc::clone(&lua_rc_outer);
        worldgen_registry_for_post
            .borrow_mut()
            .register_scripting_postprocessor(move |map| {
                let lua_rc = Rc::clone(&lua_rc_inner);
                let func: Function = lua_rc.registry_value(&func_key).unwrap();
                let map_tbl = json_to_lua_table(&lua_rc, map).unwrap();
                let map_tbl = match map_tbl {
                    mlua::Value::Table(t) => t,
                    _ => panic!("Expected Lua Table from json_to_lua_table"),
                };
                let _: () = func.call(map_tbl.clone()).unwrap(); // ok: for Lua function call
                let new_map = lua_table_to_json(&lua_rc, &map_tbl, None).unwrap(); // must be &Table
                *map = new_map;
            });
        Ok(())
    })?;
    globals.set("register_worldgen_postprocessor", register_postprocessor)?;

    Ok(())
}
