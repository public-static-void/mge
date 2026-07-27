//! Equipment set designer scripting API for Lua.
//!
//! Provides functions to load, define, apply, query, and validate equipment sets
//! backed by the engine's `EquipmentSetRegistry` and `World` loadout/validate methods.

use engine_core::ecs::EquipmentSet;
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Registers equipment set designer API functions into the Lua globals.
pub fn register_equipment_set_designer_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // load_equipment_sets(dir: string)
    let w = world.clone();
    let load_equipment_sets = lua.create_function_mut(move |_, dir: String| {
        let path = std::path::Path::new(&dir);
        let mut world = w.borrow_mut();
        world
            .load_equipment_sets(path)
            .map_err(mlua::Error::runtime)
    })?;
    globals.set("load_equipment_sets", load_equipment_sets)?;

    // define_equipment_set(name: string, items_table: table)
    let w = world.clone();
    let define_equipment_set =
        lua.create_function_mut(move |_, (name, items_table): (String, Table)| {
            let mut items = HashMap::new();
            for pair in items_table.clone().pairs::<String, String>() {
                let (slot, item_id) = pair?;
                items.insert(slot, item_id);
            }
            let set = EquipmentSet {
                name: name.clone(),
                version: "1.0.0".to_string(),
                description: String::new(),
                items,
            };
            let mut world = w.borrow_mut();
            world.equipment_set_registry.register_set(set);
            Ok(())
        })?;
    globals.set("define_equipment_set", define_equipment_set)?;

    // apply_loadout(entity: integer, set_name: string) -> integer
    let w = world.clone();
    let apply_loadout = lua.create_function_mut(
        move |_, (entity, set_name): (u32, String)| -> LuaResult<u32> {
            let mut world = w.borrow_mut();
            world
                .apply_loadout(entity, &set_name)
                .map_err(mlua::Error::runtime)
        },
    )?;
    globals.set("apply_loadout", apply_loadout)?;

    // get_loadout(entity: integer) -> table | nil
    //
    // Compares the entity's current Equipment component against all registered
    // equipment sets. Returns {name, items} of the first exact match, or nil.
    let w = world.clone();
    let get_loadout = lua.create_function_mut(move |lua, entity: u32| {
        let world = w.borrow();

        let equipment = match world.get_component(entity, "Equipment") {
            Some(e) => e,
            None => return Ok(None),
        };

        let entity_slots = match equipment.get("slots").and_then(|v| v.as_object()) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Build a map of non-null slots from the entity
        let entity_items: HashMap<&str, &str> = entity_slots
            .iter()
            .filter_map(|(slot, item_id)| item_id.as_str().map(|id| (slot.as_str(), id)))
            .collect();

        let set_names = world.equipment_set_registry.list_sets();
        for set_name in &set_names {
            if let Some(set) = world.equipment_set_registry.get_set(set_name) {
                // Exact match: same number of non-null slots, same keys, same values
                if set.items.len() == entity_items.len()
                    && set.items.iter().all(|(slot, item_id)| {
                        entity_items.get(slot.as_str()) == Some(&item_id.as_str())
                    })
                {
                    // Return {name, items} Lua table
                    let result = lua.create_table()?;
                    result.set("name", set.name.as_str())?;
                    let items_tbl = lua.create_table()?;
                    for (slot, item_id) in &set.items {
                        items_tbl.set(slot.as_str(), item_id.as_str())?;
                    }
                    result.set("items", items_tbl)?;
                    return Ok(Some(result));
                }
            }
        }

        Ok(None)
    })?;
    globals.set("get_loadout", get_loadout)?;

    // validate_equipment(entity: integer) -> table  -- { issues = { {slot, item_id, reason} } }
    let w = world.clone();
    let validate_equipment = lua.create_function_mut(move |lua, entity: u32| {
        let world = w.borrow();
        let issues = world.validate_equipment(entity);

        let result = lua.create_table()?;
        let issues_tbl = lua.create_table()?;
        for (i, issue) in issues.iter().enumerate() {
            let issue_tbl = lua.create_table()?;
            issue_tbl.set("slot", issue.slot.as_str())?;
            issue_tbl.set("item_id", issue.item_id.as_str())?;
            issue_tbl.set("reason", issue.reason.as_str())?;
            issues_tbl.set(i + 1, issue_tbl)?;
        }
        result.set("issues", issues_tbl)?;
        Ok(result)
    })?;
    globals.set("validate_equipment", validate_equipment)?;

    Ok(())
}
