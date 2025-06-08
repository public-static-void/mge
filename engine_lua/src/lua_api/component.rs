//! Component access API: set, get, remove, list, schema.

use crate::helpers::{
    json_to_lua_table, lua_error_from_any, lua_error_msg, lua_table_to_json_with_schema,
};
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_component_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // set_component(entity, name, table)
    let world_set = world.clone();
    let set_component =
        lua.create_function_mut(move |lua, (entity, name, table): (u32, String, Table)| {
            let mut world = world_set.borrow_mut();
            let schema_json = {
                let registry = world.registry.lock().unwrap();
                let schema = registry
                    .get_schema_by_name(&name)
                    .map(|s| &s.schema)
                    .ok_or_else(|| lua_error_msg(lua, "Component schema not found"))?;
                serde_json::to_value(schema)
                    .map_err(|e| lua_error_msg(lua, &format!("Schema serialization error: {e}")))?
            };
            let json_value: JsonValue = lua_table_to_json_with_schema(lua, &table, &schema_json)?;
            match world.set_component(entity, &name, json_value) {
                Ok(_) => Ok(true),
                Err(e) => Err(lua_error_from_any(lua, e)),
            }
        })?;
    globals.set("set_component", set_component)?;

    // get_component(entity, name)
    let world_get = world.clone();
    let get_component = lua.create_function_mut(move |lua, (entity, name): (u32, String)| {
        let world = world_get.borrow();
        if let Some(val) = world.get_component(entity, &name) {
            json_to_lua_table(lua, val)
        } else {
            Ok(LuaValue::Nil)
        }
    })?;
    globals.set("get_component", get_component)?;

    // remove_component(entity, name)
    let world_remove_component = world.clone();
    let remove_component = lua.create_function_mut(move |lua, (entity, name): (u32, String)| {
        let mut world = world_remove_component.borrow_mut();
        match world.remove_component(entity, &name) {
            Ok(_) => Ok(()),
            Err(e) => Err(crate::helpers::lua_error_msg(lua, &e)),
        }
    })?;
    globals.set("remove_component", remove_component)?;

    // list_components()
    let world_list_components = world.clone();
    let list_components = lua.create_function_mut(move |_, ()| {
        let world = world_list_components.borrow();
        Ok(world.registry.lock().unwrap().all_component_names())
    })?;
    globals.set("list_components", list_components)?;

    // get_component_schema(name)
    let world_get_schema = world.clone();
    let get_component_schema = lua.create_function_mut(move |lua, name: String| {
        let world = world_get_schema.borrow();
        if let Some(schema) = world.registry.lock().unwrap().get_schema_by_name(&name) {
            let json =
                serde_json::to_value(&schema.schema).map_err(|e| lua_error_from_any(lua, e))?;
            json_to_lua_table(lua, &json)
        } else {
            Err(lua_error_msg(lua, "Component schema not found"))
        }
    })?;
    globals.set("get_component_schema", get_component_schema)?;

    Ok(())
}
