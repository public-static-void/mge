use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::Value as JsonValue;

pub struct EquipmentLogicSystem;

impl System for EquipmentLogicSystem {
    fn name(&self) -> &'static str {
        "EquipmentLogicSystem"
    }

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        // For each entity with Equipment
        for eid in world.get_entities_with_component("Equipment") {
            let equipment = match world.get_component(eid, "Equipment") {
                Some(e) => e.clone(),
                None => continue,
            };

            // Get slots as a mutable object
            let slots_obj = match equipment.get("slots") {
                Some(JsonValue::Object(map)) => map.clone(),
                _ => continue,
            };

            // Collect incompatible slots to clear
            let mut slots_to_clear: Vec<String> = Vec::new();

            // For each slot, check if equipped item is compatible
            for (slot_name, item_id_value) in &slots_obj {
                if item_id_value.is_null() {
                    continue;
                }
                let item_id = match item_id_value.as_str() {
                    Some(id) => id,
                    None => continue,
                };

                // Find the item entity with this id
                let mut item_metadata_opt = None;
                for item_eid in world.get_entities_with_component("Item") {
                    if let Some(item_comp) = world.get_component(item_eid, "Item") {
                        if let Some(id_val) = item_comp.get("id") {
                            if id_val == item_id {
                                item_metadata_opt = Some(item_comp.clone());
                                break;
                            }
                        }
                    }
                }
                let item_metadata = match item_metadata_opt {
                    Some(meta) => meta,
                    None => continue,
                };

                // Check slot compatibility
                if let Some(item_slot) = item_metadata.get("slot").and_then(|v| v.as_str()) {
                    if item_slot != slot_name {
                        slots_to_clear.push(slot_name.clone());
                    }
                }
            }

            // If any slots need to be cleared, update the Equipment component
            if !slots_to_clear.is_empty() {
                let mut new_equipment = equipment.clone();
                if let Some(JsonValue::Object(slots)) = new_equipment.get_mut("slots") {
                    for slot in slots_to_clear {
                        slots.insert(slot, JsonValue::Null);
                    }
                }
                let _ = world.set_component(eid, "Equipment", new_equipment);
            }
        }
    }
}
