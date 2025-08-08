//! Equipment helpers for scripting API.

use crate::helpers::{
    ensure_schema_arrays, json_to_lua_table, lua_error_from_any, lua_error_msg, lua_table_to_json,
};
use crate::schemas::get_schema;
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_equipment_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_equipment(entity)
    let world_get_equipment = world.clone();
    let get_equipment = lua.create_function_mut(move |lua, entity: u32| {
        let world = world_get_equipment.borrow();
        if let Some(mut val) = world.get_component(entity, "Equipment").cloned() {
            let schema = get_schema("Equipment").expect("Equipment schema missing");
            ensure_schema_arrays(&mut val, schema);
            json_to_lua_table(lua, &val)
        } else {
            Ok(mlua::Value::Table(lua.create_table()?))
        }
    })?;
    globals.set("get_equipment", get_equipment)?;

    // set_equipment(entity, table)
    let world_set_equipment = world.clone();
    let set_equipment = lua.create_function_mut(move |lua, (entity, table): (u32, Table)| {
        let mut world = world_set_equipment.borrow_mut();
        let mut json_value: JsonValue = lua_table_to_json(lua, &table, None)?;
        let schema = get_schema("Equipment").expect("Equipment schema missing");
        ensure_schema_arrays(&mut json_value, schema);
        world
            .set_component(entity, "Equipment", json_value)
            .map_err(|e| lua_error_from_any(lua, e))
    })?;
    globals.set("set_equipment", set_equipment)?;

    // equip_item(entity, item_id, slot)
    let world_equip_item = world.clone();
    let equip_item =
        lua.create_function_mut(move |lua, (entity, item_id, slot): (u32, String, String)| {
            let mut world = world_equip_item.borrow_mut();

            // 1. Check inventory
            let inv = world
                .get_component(entity, "Inventory")
                .ok_or_else(|| lua_error_msg(lua, "Entity has no Inventory"))?;
            let slots = inv
                .get("slots")
                .and_then(|v| v.as_array())
                .ok_or_else(|| lua_error_msg(lua, "No slots array in Inventory"))?;
            if !slots
                .iter()
                .any(|v| v == &serde_json::Value::String(item_id.clone()))
            {
                return Err(lua_error_msg(lua, "Item not in Inventory"));
            }

            // 2. Check item metadata
            let mut found = None;
            for item_eid in world.get_entities_with_component("Item") {
                if let Some(item_comp) = world.get_component(item_eid, "Item")
                    && item_comp.get("id") == Some(&serde_json::Value::String(item_id.clone()))
                {
                    found = Some(item_comp);
                    break;
                }
            }
            let item_meta = found.ok_or_else(|| lua_error_msg(lua, "Item not found"))?;

            // 3. Check slot compatibility
            let valid_slot = item_meta
                .get("slot")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lua_error_msg(lua, "Item missing slot info"))?;
            if valid_slot != slot {
                return Err(lua_error_msg(lua, "Invalid slot"));
            }

            // 4. Get or create Equipment component with correct structure
            let mut equipment = if let Some(val) = world.get_component(entity, "Equipment") {
                val.clone()
            } else {
                let mut map = serde_json::Map::new();
                map.insert("slots".to_string(), serde_json::json!({}));
                serde_json::Value::Object(map)
            };

            // 5. Ensure "slots" is an object
            let slots_obj = equipment
                .get_mut("slots")
                .and_then(|v| v.as_object_mut())
                .ok_or_else(|| lua_error_msg(lua, "Equipment slots must be an object"))?;

            // 6. Check if slot is already occupied
            if let Some(existing) = slots_obj.get(&slot)
                && !existing.is_null()
            {
                return Err(lua_error_msg(lua, "Slot already occupied"));
            }

            // 7. Equip
            slots_obj.insert(slot.clone(), serde_json::Value::String(item_id.clone()));

            // 8. Schema enforcement
            let schema = get_schema("Equipment").expect("Equipment schema missing");
            ensure_schema_arrays(&mut equipment, schema);

            world
                .set_component(entity, "Equipment", equipment)
                .map_err(|e| lua_error_from_any(lua, e))
        })?;
    globals.set("equip_item", equip_item)?;

    // unequip_item(entity, slot)
    let world_unequip_item = world.clone();
    let unequip_item = lua.create_function_mut(move |lua, (entity, slot): (u32, String)| {
        let mut world = world_unequip_item.borrow_mut();
        let mut equipment = world
            .get_component(entity, "Equipment")
            .ok_or_else(|| lua_error_msg(lua, "No Equipment component"))?
            .clone();
        let slots_obj = equipment
            .get_mut("slots")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| lua_error_msg(lua, "Equipment slots must be an object"))?;
        slots_obj.insert(slot, serde_json::Value::Null);

        // Schema enforcement
        let schema = get_schema("Equipment").expect("Equipment schema missing");
        ensure_schema_arrays(&mut equipment, schema);

        world
            .set_component(entity, "Equipment", equipment)
            .map_err(|e| lua_error_from_any(lua, e))
    })?;
    globals.set("unequip_item", unequip_item)?;

    Ok(())
}
