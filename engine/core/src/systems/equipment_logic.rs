use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

/// Global cache for valid equipment slot names, loaded from equipment_slots.json.
fn get_valid_slots() -> &'static HashSet<String> {
    static SLOTS: OnceLock<HashSet<String>> = OnceLock::new();
    SLOTS.get_or_init(|| {
        let mut slots = HashSet::new();
        // Try loading from default schema path
        let paths = [
            "engine/assets/schemas/equipment_slots.json",
            "../engine/assets/schemas/equipment_slots.json",
        ];
        for path_str in &paths {
            let path = Path::new(path_str);
            if path.exists() {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        if let Ok(json) = serde_json::from_str::<JsonValue>(&content)
                            && let Some(slot_list) = json.get("slots").and_then(|v| v.as_array())
                        {
                            for slot_val in slot_list {
                                if let Some(s) = slot_val.as_str() {
                                    slots.insert(s.to_string());
                                }
                            }
                        }
                    }
                    Err(_) => continue,
                }
                break;
            }
        }
        slots
    })
}

///Equipment logic system
/// Validates slot compatibility, stat requirements, and two-handed weapon enforcement.
/// Does NOT write to Stats or EquipmentEffects (responsibility of downstream systems).
pub struct EquipmentLogicSystem;

impl System for EquipmentLogicSystem {
    fn name(&self) -> &'static str {
        "EquipmentLogicSystem"
    }

    fn run(&mut self, world: &mut World) {
        for eid in world.get_entities_with_component("Equipment") {
            let equipment = match world.get_component(eid, "Equipment") {
                Some(e) => e.clone(),
                None => continue,
            };

            let slots_obj = match equipment.get("slots") {
                Some(JsonValue::Object(map)) => map.clone(),
                _ => continue,
            };

            // Cache item metadata by item ID for performance
            let mut item_cache: HashMap<String, JsonValue> = HashMap::new();

            // Get entity stats for requirement checks
            let empty = JsonValue::Object(Default::default());
            let entity_stats = world.get_component(eid, "Stats").unwrap_or(&empty);

            // Collect changes to apply after validation
            let mut new_equipment = equipment.clone();
            let slots_mut = match new_equipment
                .get_mut("slots")
                .and_then(JsonValue::as_object_mut)
            {
                Some(s) => s,
                None => continue,
            };

            // Helper closure to get item metadata by ID
            let mut get_item_metadata = |item_id: &str| -> Option<JsonValue> {
                if let Some(cached) = item_cache.get(item_id) {
                    return Some(cached.clone());
                }
                for item_eid in world.get_entities_with_component("Item") {
                    if let Some(item_comp) = world.get_component(item_eid, "Item")
                        && let Some(id_val) = item_comp.get("id")
                        && id_val == item_id
                    {
                        item_cache.insert(item_id.to_string(), item_comp.clone());
                        return Some(item_comp.clone());
                    }
                }
                None
            };

            // First pass: enforce slot compatibility, stat requirements, and slot registry validation
            let valid_slots = get_valid_slots();
            for (slot_name, item_id_value) in &slots_obj {
                if item_id_value.is_null() {
                    continue;
                }
                let item_id = match item_id_value.as_str() {
                    Some(id) => id,
                    None => continue,
                };

                let item_metadata = match get_item_metadata(item_id) {
                    Some(meta) => meta,
                    None => continue,
                };

                // Slot registry validation: reject slots not in the valid set
                if !valid_slots.is_empty() && !valid_slots.contains(slot_name.as_str()) {
                    log::warn!(
                        "Invalid slot '{}' for item '{}' — not in equipment_slots registry",
                        slot_name,
                        item_id
                    );
                    slots_mut.insert(slot_name.clone(), JsonValue::Null);
                    continue;
                }

                // Check slot compatibility
                if let Some(item_slot) = item_metadata.get("slot").and_then(|v| v.as_str())
                    && item_slot != slot_name
                {
                    // Incompatible slot: unequip
                    slots_mut.insert(slot_name.clone(), JsonValue::Null);
                    continue;
                }

                // Check stat requirements
                if let Some(requirements) = item_metadata
                    .get("requirements")
                    .and_then(JsonValue::as_object)
                {
                    let mut unmet = false;
                    for (req_key, req_val) in requirements {
                        if let (Some(req_num), Some(stat_val)) = (
                            req_val.as_i64(),
                            entity_stats.get(req_key).and_then(JsonValue::as_i64),
                        ) && stat_val < req_num
                        {
                            unmet = true;
                            break;
                        }
                    }
                    if unmet {
                        // Requirements not met: unequip
                        slots_mut.insert(slot_name.clone(), JsonValue::Null);
                        continue;
                    }
                }
            }

            // Second pass: handle two-handed weapons (same logic, no stat writing)
            // Uses the mutable slots_mut for checking current state
            let current_slots: Vec<(String, Option<String>)> = slots_mut
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().map(|s| s.to_string())))
                .collect();

            let mut two_handed_items: Vec<(String, String)> = Vec::new();
            for (slot_name, item_id_opt) in &current_slots {
                let item_id = match item_id_opt {
                    Some(id) => id.clone(),
                    None => continue,
                };
                let item_metadata = match get_item_metadata(&item_id) {
                    Some(meta) => meta,
                    None => continue,
                };
                if item_metadata.get("two_handed").and_then(JsonValue::as_bool) == Some(true) {
                    two_handed_items.push((slot_name.clone(), item_id));
                }
            }

            if !two_handed_items.is_empty() {
                let (_two_handed_slot, two_handed_id) = &two_handed_items[0];
                let hands = ["left_hand", "right_hand"];
                let mut conflict = false;
                for hand in hands.iter() {
                    if let Some(item_val) = slots_mut.get(*hand) {
                        if item_val.is_null() {
                            continue;
                        }
                        if item_val.as_str() != Some(two_handed_id) {
                            conflict = true;
                            break;
                        }
                    }
                }
                if conflict {
                    for (slot, _) in &two_handed_items {
                        slots_mut.insert(slot.clone(), JsonValue::Null);
                    }
                } else {
                    for hand in hands.iter() {
                        slots_mut
                            .insert(hand.to_string(), JsonValue::String(two_handed_id.clone()));
                    }
                }
            }

            // Apply equipment changes only (no stat writes)
            let _ = world.set_component(eid, "Equipment", new_equipment);
        }
    }
}
