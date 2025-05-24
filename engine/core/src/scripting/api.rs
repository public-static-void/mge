use crate::ecs::world::World;
use crate::scripting::helpers::{
    json_to_lua_table, lua_error_from_any, lua_error_msg, lua_table_to_json,
    lua_table_to_json_with_schema,
};
use crate::scripting::input::InputProvider;
use crate::systems::standard::{DamageAll, MoveAll, MoveDelta, ProcessDeaths, ProcessDecay};
use crate::worldgen::WorldgenRegistry;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn register_api_functions(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
    input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
) -> LuaResult<()> {
    // spawn_entity()
    let world_spawn = world.clone();
    let spawn_entity = lua.create_function_mut(move |_, ()| {
        let mut world = world_spawn.borrow_mut();
        Ok(world.spawn_entity())
    })?;
    globals.set("spawn_entity", spawn_entity)?;

    // set_component(entity, name, table)
    let world_set = world.clone();
    let set_component =
        lua.create_function_mut(move |lua, (entity, name, table): (u32, String, Table)| {
            let mut world = world_set.borrow_mut();
            // Step 1: Get the schema JSON in its own scope
            let schema_json = {
                let registry = world.registry.lock().unwrap();
                let schema = registry
                    .get_schema_by_name(&name)
                    .map(|s| &s.schema)
                    .ok_or_else(|| lua_error_msg(lua, "Component schema not found"))?;
                serde_json::to_value(schema)
                    .map_err(|e| lua_error_msg(lua, &format!("Schema serialization error: {e}")))?
            }; // <-- lock is dropped here

            // Step 2: Now you can mutably borrow world
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
    let remove_component = lua.create_function_mut(move |_, (entity, name): (u32, String)| {
        let mut world = world_remove_component.borrow_mut();
        if let Some(comps) = world.components.get_mut(&name) {
            comps.remove(&entity);
        }
        Ok(())
    })?;
    globals.set("remove_component", remove_component)?;

    // set_mode(mode: String)
    let world_set_mode = world.clone();
    let set_mode = lua.create_function_mut(move |_, mode: String| {
        let mut world = world_set_mode.borrow_mut();
        world.current_mode = mode;
        Ok(())
    })?;
    globals.set("set_mode", set_mode)?;

    // get_mode()
    let world_get_mode = world.clone();
    let get_mode = lua.create_function_mut(move |_, ()| {
        let world = world_get_mode.borrow();
        Ok(world.current_mode.clone())
    })?;
    globals.set("get_mode", get_mode)?;

    // get_available_modes()
    let world_get_modes = world.clone();
    let get_available_modes = lua.create_function_mut(move |_, ()| {
        let world = world_get_modes.borrow();
        let modes = world.registry.lock().unwrap().all_modes();
        Ok(modes.into_iter().collect::<Vec<String>>())
    })?;
    globals.set("get_available_modes", get_available_modes)?;

    // get_entities()
    let world_get_entities = world.clone();
    let get_entities = lua.create_function_mut(move |_, ()| {
        let world = world_get_entities.borrow();
        Ok(world.entities.clone())
    })?;
    globals.set("get_entities", get_entities)?;

    // get_entities_with_component(name)
    let world_get_entities = world.clone();
    let get_entities_with_component = lua.create_function_mut(move |_, name: String| {
        let world = world_get_entities.borrow();
        let ids = world.get_entities_with_component(&name);
        Ok(ids)
    })?;
    globals.set("get_entities_with_component", get_entities_with_component)?;

    // get_entities_with_components(names)
    let world_query = world.clone();
    let get_entities_with_components = lua.create_function_mut(move |_lua, names: Table| {
        let world = world_query.borrow();
        let mut rust_names = Vec::new();
        for pair in names.sequence_values::<String>() {
            rust_names.push(pair?);
        }
        let name_refs: Vec<&str> = rust_names.iter().map(|s| s.as_str()).collect();
        Ok(world.get_entities_with_components(&name_refs))
    })?;
    globals.set("get_entities_with_components", get_entities_with_components)?;

    // move_entity(entity, dx, dy)
    let world_move_entity = world.clone();
    let move_entity = lua.create_function_mut(move |_, (entity, dx, dy): (u32, f32, f32)| {
        let mut world = world_move_entity.borrow_mut();
        world.move_entity(entity, dx, dy);
        Ok(())
    })?;
    globals.set("move_entity", move_entity)?;

    // move_all(dx, dy)
    let world_move = world.clone();
    let move_all = lua.create_function_mut(move |_, (dx, dy): (i32, i32)| {
        let mut world = world_move.borrow_mut();
        world.register_system(MoveAll {
            delta: MoveDelta::Square { dx, dy, dz: 0 },
        });
        world.run_system("MoveAll", None).unwrap();
        Ok(())
    })?;
    globals.set("move_all", move_all)?;

    // damage_entity(entity, amount)
    let world_damage_entity = world.clone();
    let damage_entity = lua.create_function_mut(move |_, (entity, amount): (u32, f32)| {
        let mut world = world_damage_entity.borrow_mut();
        world.damage_entity(entity, amount);
        Ok(())
    })?;
    globals.set("damage_entity", damage_entity)?;

    // damage_all(amount)
    let world_damage = world.clone();
    let damage_all = lua.create_function_mut(move |_, amount: f32| {
        let mut world = world_damage.borrow_mut();
        world.register_system(DamageAll { amount });
        world.run_system("DamageAll", None).unwrap();
        Ok(())
    })?;
    globals.set("damage_all", damage_all)?;

    // tick()
    let world_tick = world.clone();
    let tick = lua.create_function_mut(move |_, ()| {
        let mut world = world_tick.borrow_mut();
        world.run_system("MoveAll", None).unwrap();
        world.run_system("DamageAll", None).unwrap();
        world.run_system("ProcessDeaths", None).unwrap();
        world.run_system("ProcessDecay", None).unwrap();
        world.turn += 1;
        Ok(())
    })?;
    globals.set("tick", tick)?;

    // get_turn()
    let world_get_turn = world.clone();
    let get_turn = lua.create_function_mut(move |_, ()| {
        let world = world_get_turn.borrow();
        Ok(world.turn)
    })?;
    globals.set("get_turn", get_turn)?;

    // process_deaths()
    let world_deaths = world.clone();
    let process_deaths = lua.create_function_mut(move |_, ()| {
        let mut world = world_deaths.borrow_mut();
        world.register_system(ProcessDeaths);
        world.run_system("ProcessDeaths", None).unwrap();
        Ok(())
    })?;
    globals.set("process_deaths", process_deaths)?;

    // process_decay()
    let world_decay = world.clone();
    let process_decay = lua.create_function_mut(move |_, ()| {
        let mut world = world_decay.borrow_mut();
        world.register_system(ProcessDecay);
        world.run_system("ProcessDecay", None).unwrap();
        Ok(())
    })?;
    globals.set("process_decay", process_decay)?;

    // despawn_entity(id)
    let world_remove = world.clone();
    let despawn_entity = lua.create_function_mut(move |_, entity_id: u32| {
        let mut world = world_remove.borrow_mut();
        world.despawn_entity(entity_id);
        Ok(())
    })?;
    globals.set("despawn_entity", despawn_entity)?;

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

    // is_entity_alive(entity)
    let world_is_alive = world.clone();
    let is_entity_alive = lua.create_function_mut(move |_, entity: u32| {
        let world = world_is_alive.borrow();
        Ok(world.is_entity_alive(entity))
    })?;
    globals.set("is_entity_alive", is_entity_alive)?;

    // get_user_input(prompt)
    let input_provider_clone = input_provider.clone();
    let get_user_input = lua.create_function(move |lua, prompt: String| {
        let mut provider = input_provider_clone
            .lock()
            .map_err(|_| lua_error_msg(lua, "Input provider lock poisoned"))?;
        provider
            .read_line(&prompt)
            .map_err(|e| lua_error_msg(lua, &format!("Input error: {e}")))
    })?;
    globals.set("get_user_input", get_user_input)?;

    let world_count_type = world.clone();
    let count_entities_with_type = lua.create_function_mut(move |_, type_str: String| {
        let world = world_count_type.borrow();
        Ok(world.count_entities_with_type(&type_str))
    })?;
    globals.set("count_entities_with_type", count_entities_with_type)?;

    let world_modify_stockpile = world.clone();
    let modify_stockpile_resource =
        lua.create_function_mut(move |lua, (entity, kind, delta): (u32, String, f64)| {
            let mut world = world_modify_stockpile.borrow_mut();
            world
                .modify_stockpile_resource(entity, &kind, delta)
                .map_err(|e| lua_error_from_any(lua, e))
        })?;
    globals.set("modify_stockpile_resource", modify_stockpile_resource)?;

    // save_world(filename)
    let world_save = world.clone();
    let save_to_file = lua.create_function_mut(move |lua, filename: String| {
        let world = world_save.borrow();
        world
            .save_to_file(std::path::Path::new(&filename))
            .map_err(|e| lua_error_from_any(lua, e))
    })?;
    globals.set("save_to_file", save_to_file)?;

    // load_world(filename)
    let world_load = world.clone();
    let registry = world.borrow().registry.clone();
    let load_from_file = lua.create_function_mut(move |lua, filename: String| {
        let mut world = world_load.borrow_mut();
        let loaded = World::load_from_file(std::path::Path::new(&filename), registry.clone())
            .map_err(|e| lua_error_from_any(lua, e))?;
        *world = loaded;
        Ok(())
    })?;
    globals.set("load_from_file", load_from_file)?;

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

    // add_item_to_inventory(entity, item)
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

    // get_equipment(entity)
    let world_get_equipment = world.clone();
    let get_equipment = lua.create_function_mut(move |lua, entity: u32| {
        let world = world_get_equipment.borrow();
        if let Some(val) = world.get_component(entity, "Equipment") {
            json_to_lua_table(lua, val)
        } else {
            // Always return an empty table if not present
            Ok(mlua::Value::Table(lua.create_table()?))
        }
    })?;
    globals.set("get_equipment", get_equipment)?;

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
                if let Some(item_comp) = world.get_component(item_eid, "Item") {
                    if item_comp.get("id") == Some(&serde_json::Value::String(item_id.clone())) {
                        found = Some(item_comp);
                        break;
                    }
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
                // Always create with correct structure
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
            if let Some(existing) = slots_obj.get(&slot) {
                if !existing.is_null() {
                    return Err(lua_error_msg(lua, "Slot already occupied"));
                }
            }

            // 7. Equip
            slots_obj.insert(slot.clone(), serde_json::Value::String(item_id.clone()));
            let result = world.set_component(entity, "Equipment", equipment);
            println!("set_component result: {:?}", result);
            result.map_err(|e| lua_error_from_any(lua, e))
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
        world
            .set_component(entity, "Equipment", equipment)
            .map_err(|e| lua_error_from_any(lua, e))
    })?;
    globals.set("unequip_item", unequip_item)?;

    // get_entities_in_region(region_id)
    let world_entities_in_region = world.clone();
    let get_entities_in_region = lua.create_function_mut(move |_, region_id: String| {
        let world = world_entities_in_region.borrow();
        Ok(world.entities_in_region(&region_id))
    })?;
    globals.set("get_entities_in_region", get_entities_in_region)?;

    // get_entities_in_region_kind(kind)
    let world_entities_in_region_kind = world.clone();
    let get_entities_in_region_kind = lua.create_function_mut(move |_, kind: String| {
        let world = world_entities_in_region_kind.borrow();
        Ok(world.entities_in_region_kind(&kind))
    })?;
    globals.set("get_entities_in_region_kind", get_entities_in_region_kind)?;

    // get_cells_in_region(region_id)
    let world_cells_in_region = world.clone();
    let get_cells_in_region = lua.create_function_mut(move |lua, region_id: String| {
        let world = world_cells_in_region.borrow();
        let cells = world.cells_in_region(&region_id);
        json_to_lua_table(lua, &serde_json::Value::Array(cells))
    })?;
    globals.set("get_cells_in_region", get_cells_in_region)?;

    // get_cells_in_region_kind(kind)
    let world_cells_in_region_kind = world.clone();
    let get_cells_in_region_kind = lua.create_function_mut(move |lua, kind: String| {
        let world = world_cells_in_region_kind.borrow();
        let cells = world.cells_in_region_kind(&kind);
        json_to_lua_table(lua, &serde_json::Value::Array(cells))
    })?;
    globals.set("get_cells_in_region_kind", get_cells_in_region_kind)?;

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

            // Recursive helper to remove part by name from an array of parts
            fn remove_part_recursive(parts: &mut Vec<serde_json::Value>, name: &str) -> bool {
                let mut i = 0;
                while i < parts.len() {
                    let part = &mut parts[i];
                    // Check if this part matches
                    if part.get("name").and_then(|n| n.as_str()) == Some(name) {
                        parts.remove(i);
                        return true;
                    }
                    // Otherwise, check children recursively
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

/// Registers worldgen functions to Lua:
/// - list_worldgen_plugins() -> {"plugin1", "plugin2", ... }
/// - invoke_worldgen(name, params_table) -> map_table
pub fn register_worldgen_api(
    lua: &Lua,
    globals: &Table,
    worldgen_registry: Rc<WorldgenRegistry>,
) -> LuaResult<()> {
    let registry_for_list = Rc::clone(&worldgen_registry);
    let list_plugins = lua.create_function(move |_, ()| {
        let plugins = registry_for_list.list_names();
        Ok(plugins)
    })?;
    globals.set("list_worldgen_plugins", list_plugins)?;

    let registry_for_invoke = Rc::clone(&worldgen_registry);
    let invoke_worldgen = lua.create_function(move |lua, (name, params): (String, Table)| {
        let params_json: JsonValue = lua_table_to_json(lua, &params, None)?;
        let result = registry_for_invoke
            .invoke(&name, &params_json)
            .map_err(|e| lua_error_from_any(lua, e))?;
        json_to_lua_table(lua, &result)
    })?;
    globals.set("invoke_worldgen", invoke_worldgen)?;

    Ok(())
}
