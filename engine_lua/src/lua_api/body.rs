//! Body helpers for scripting API.

use crate::helpers::{json_to_lua_table, lua_error_from_any, lua_error_msg, lua_table_to_json};
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_body_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // get_body(entity)
    let world_get_body = world.clone();
    let get_body = lua.create_function_mut(move |lua, entity: u32| {
        let world = world_get_body.borrow();
        if let Some(val) = world.get_component(entity, "Body") {
            json_to_lua_table(lua, val)
        } else {
            Ok(mlua::Value::Nil)
        }
    })?;
    globals.set("get_body", get_body)?;

    // set_body(entity, table)
    let world_set_body = world.clone();
    let set_body = lua.create_function_mut(move |lua, (entity, table): (u32, Table)| {
        let mut world = world_set_body.borrow_mut();
        let json_value: JsonValue = lua_table_to_json(lua, &table, None)?;
        world
            .set_component(entity, "Body", json_value)
            .map_err(|e| lua_error_from_any(lua, e))
    })?;
    globals.set("set_body", set_body)?;

    // add_body_part(entity, part_table)
    let world_add_body_part = world.clone();
    let add_body_part = lua.create_function_mut(move |lua, (entity, part): (u32, Table)| {
        let mut world = world_add_body_part.borrow_mut();
        let mut body = if let Some(val) = world.get_component(entity, "Body") {
            val.clone()
        } else {
            serde_json::json!({})
        };
        let parts = body.get_mut("parts").and_then(|v| v.as_array_mut());
        let parts = if let Some(parts) = parts {
            parts
        } else {
            body["parts"] = serde_json::json!([]);
            body.get_mut("parts").unwrap().as_array_mut().unwrap()
        };
        let part_json: JsonValue = lua_table_to_json(lua, &part, None)?;
        parts.push(part_json);
        world
            .set_component(entity, "Body", body)
            .map_err(|e| lua_error_from_any(lua, e))
    })?;
    globals.set("add_body_part", add_body_part)?;

    // remove_body_part(entity, part_name)
    let world_remove_body_part = world.clone();
    let remove_body_part =
        lua.create_function_mut(move |lua, (entity, part_name): (u32, String)| {
            let mut world = world_remove_body_part.borrow_mut();
            let mut body = if let Some(val) = world.get_component(entity, "Body") {
                val.clone()
            } else {
                return Err(lua_error_msg(lua, "No Body component found"));
            };

            fn remove_part_recursive(parts: &mut Vec<serde_json::Value>, name: &str) -> bool {
                let mut i = 0;
                while i < parts.len() {
                    let part = &mut parts[i];
                    if part.get("name").and_then(|n| n.as_str()) == Some(name) {
                        parts.remove(i);
                        return true;
                    }
                    if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut())
                    {
                        if remove_part_recursive(children, name) {
                            return true;
                        }
                    }
                    i += 1;
                }
                false
            }

            if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
                if remove_part_recursive(parts, &part_name) {
                    world
                        .set_component(entity, "Body", body)
                        .map_err(|e| lua_error_from_any(lua, e))
                } else {
                    Err(lua_error_msg(lua, "Body part not found"))
                }
            } else {
                Err(lua_error_msg(lua, "No parts array in Body"))
            }
        })?;
    globals.set("remove_body_part", remove_body_part)?;

    // get_body_part(entity, part_name)
    let world_get_body_part = world.clone();
    let get_body_part =
        lua.create_function_mut(move |lua, (entity, part_name): (u32, String)| {
            let world = world_get_body_part.borrow();
            if let Some(body) = world.get_component(entity, "Body") {
                if let Some(parts) = body.get("parts").and_then(|v| v.as_array()) {
                    for part in parts {
                        if part.get("name").and_then(|n| n.as_str()) == Some(&part_name) {
                            return json_to_lua_table(lua, part);
                        }
                    }
                }
            }
            Ok(mlua::Value::Nil)
        })?;
    globals.set("get_body_part", get_body_part)?;

    Ok(())
}
