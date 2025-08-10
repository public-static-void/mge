//! Body helpers for scripting API.

use crate::helpers::{
    ensure_schema_arrays, json_to_lua_table, lua_error_from_any, lua_error_msg, lua_table_to_json,
};
use crate::schemas::get_schema;
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the body API.
pub fn register_body_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // get_body(entity)
    let world_get_body = world.clone();
    let get_body = lua.create_function_mut(move |lua, entity: u32| {
        let world = world_get_body.borrow();
        if let Some(mut val) = world.get_component(entity, "Body").cloned() {
            let schema = get_schema("Body").expect("Body schema missing");
            ensure_schema_arrays(&mut val, schema);
            json_to_lua_table(lua, &val)
        } else {
            Ok(mlua::Value::Nil)
        }
    })?;
    globals.set("get_body", get_body)?;

    // set_body(entity, table)
    let world_set_body = world.clone();
    let set_body = lua.create_function_mut(move |lua, (entity, table): (u32, Table)| {
        let mut world = world_set_body.borrow_mut();
        let mut json_value: JsonValue = lua_table_to_json(lua, &table, None)?;
        let schema = get_schema("Body").expect("Body schema missing");
        ensure_schema_arrays(&mut json_value, schema);
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
        let mut part_json: JsonValue = lua_table_to_json(lua, &part, None)?;
        let schema = get_schema("Body").expect("Body schema missing");
        ensure_schema_arrays(
            &mut part_json,
            schema
                .get("properties")
                .and_then(|p| p.get("parts"))
                .and_then(|s| s.get("items"))
                .unwrap_or(schema),
        );
        parts.push(part_json);
        ensure_schema_arrays(&mut body, schema);
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
                        && remove_part_recursive(children, name)
                    {
                        return true;
                    }
                    i += 1;
                }
                false
            }

            if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
                if remove_part_recursive(parts, &part_name) {
                    let schema = get_schema("Body").expect("Body schema missing");
                    ensure_schema_arrays(&mut body, schema);

                    fn fix_empty_arrays(part: &mut serde_json::Value) {
                        if let Some(children) = part.get_mut("children") {
                            if children.is_null()
                                || (children.is_array() && children.as_array().unwrap().is_empty())
                            {
                                *children = serde_json::json!([]);
                            } else if let Some(arr) = children.as_array_mut() {
                                for child in arr {
                                    fix_empty_arrays(child);
                                }
                            }
                        }
                        if let Some(equipped) = part.get_mut("equipped")
                            && (equipped.is_null()
                                || (equipped.is_array() && equipped.as_array().unwrap().is_empty()))
                        {
                            *equipped = serde_json::json!([]);
                        }
                    }

                    if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
                        for part in parts {
                            fix_empty_arrays(part);
                        }
                    }

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
            if let Some(mut body) = world.get_component(entity, "Body").cloned() {
                let schema = get_schema("Body").expect("Body schema missing");
                ensure_schema_arrays(&mut body, schema);
                if let Some(parts) = body.get("parts").and_then(|v| v.as_array()) {
                    for part in parts {
                        if part.get("name").and_then(|n| n.as_str()) == Some(&part_name) {
                            // Ensure part is schema-compliant before returning
                            let mut part_clone = part.clone();
                            ensure_schema_arrays(
                                &mut part_clone,
                                schema
                                    .get("properties")
                                    .and_then(|p| p.get("parts"))
                                    .and_then(|s| s.get("items"))
                                    .unwrap_or(schema),
                            );
                            return json_to_lua_table(lua, &part_clone);
                        }
                    }
                }
            }
            Ok(mlua::Value::Nil)
        })?;
    globals.set("get_body_part", get_body_part)?;

    Ok(())
}
