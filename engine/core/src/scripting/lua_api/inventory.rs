//! Inventory helpers for scripting API.

use crate::ecs::world::World;
use crate::scripting::helpers::{
    json_to_lua_table, lua_error_from_any, lua_error_msg, lua_table_to_json,
};
use mlua::{Lua, Result as LuaResult, Table};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_inventory_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_inventory(entity)
    let world_get_inventory = world.clone();
    let get_inventory = lua.create_function_mut(move |lua, entity: u32| {
        let world = world_get_inventory.borrow();
        if let Some(val) = world.get_component(entity, "Inventory") {
            json_to_lua_table(lua, val)
        } else {
            Ok(mlua::Value::Nil)
        }
    })?;
    globals.set("get_inventory", get_inventory)?;

    // set_inventory(entity, table)
    let world_set_inventory = world.clone();
    let set_inventory = lua.create_function_mut(move |lua, (entity, table): (u32, Table)| {
        let mut world = world_set_inventory.borrow_mut();
        let json_value: JsonValue = lua_table_to_json(lua, &table, None)?;
        world
            .set_component(entity, "Inventory", json_value)
            .map_err(|e| lua_error_from_any(lua, e))
    })?;
    globals.set("set_inventory", set_inventory)?;

    // add_item_to_inventory(entity, item_id)
    let world_add_item = world.clone();
    let add_item_to_inventory =
        lua.create_function_mut(move |lua, (entity, item_id): (u32, String)| {
            let mut world = world_add_item.borrow_mut();
            let mut inv = if let Some(val) = world.get_component(entity, "Inventory") {
                val.clone()
            } else {
                serde_json::json!({})
            };
            let slots_opt = inv.get_mut("slots").and_then(|v| v.as_array_mut());
            let slots = if let Some(slots) = slots_opt {
                slots
            } else {
                inv["slots"] = serde_json::json!([]);
                inv.get_mut("slots").unwrap().as_array_mut().unwrap()
            };
            slots.push(serde_json::Value::String(item_id));
            world
                .set_component(entity, "Inventory", inv)
                .map_err(|e| lua_error_from_any(lua, e))
        })?;
    globals.set("add_item_to_inventory", add_item_to_inventory)?;

    // remove_item_from_inventory(entity, index)
    let world_remove_item = world.clone();
    let remove_item_from_inventory =
        lua.create_function_mut(move |lua, (entity, index): (u32, usize)| {
            let mut world = world_remove_item.borrow_mut();
            let mut inv = if let Some(val) = world.get_component(entity, "Inventory") {
                val.clone()
            } else {
                return Err(lua_error_msg(lua, "No Inventory component found"));
            };
            if let Some(slots) = inv.get_mut("slots").and_then(|v| v.as_array_mut()) {
                if index < slots.len() {
                    slots.remove(index);
                    world
                        .set_component(entity, "Inventory", inv)
                        .map_err(|e| lua_error_from_any(lua, e))
                } else {
                    Err(lua_error_msg(lua, "Index out of bounds"))
                }
            } else {
                Err(lua_error_msg(lua, "No slots array in Inventory"))
            }
        })?;
    globals.set("remove_item_from_inventory", remove_item_from_inventory)?;

    Ok(())
}
