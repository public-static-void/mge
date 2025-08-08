use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

pub struct EquipmentLogicSystem;

impl System for EquipmentLogicSystem {
    fn name(&self) -> &'static str {
        "EquipmentLogicSystem"
    }

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
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
            let slots_mut = new_equipment
                .get_mut("slots")
                .and_then(JsonValue::as_object_mut);
            if slots_mut.is_none() {
                continue;
            }
            let slots_mut = slots_mut.unwrap();

            // Helper closure to get item metadata by ID
            let mut get_item_metadata = |item_id: &str| -> Option<JsonValue> {
                if let Some(cached) = item_cache.get(item_id) {
                    return Some(cached.clone());
                }
                for item_eid in world.get_entities_with_component("Item") {
                    if let Some(item_comp) = world.get_component(item_eid, "Item")
                        && let Some(id_val) = item_comp.get("id")
                            && id_val == item_id {
                                item_cache.insert(item_id.to_string(), item_comp.clone());
                                return Some(item_comp.clone());
                            }
                }
                None
            };

            // First pass: enforce slot compatibility and stat requirements
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

                // Check slot compatibility
                if let Some(item_slot) = item_metadata.get("slot").and_then(|v| v.as_str())
                    && item_slot != slot_name {
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
                        )
                            && stat_val < req_num {
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

            // Second pass: handle two-handed weapons
            let mut two_handed_items: Vec<(String, String)> = Vec::new(); // (slot_name, item_id)
            for (slot_name, item_id_value) in slots_mut.iter() {
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
                if item_metadata.get("two_handed").and_then(JsonValue::as_bool) == Some(true) {
                    two_handed_items.push((slot_name.clone(), item_id.to_string()));
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

            // --- Equipment Effects Application ---
            let mut total_effects: HashMap<String, f64> = HashMap::new();
            for (_slot_name, item_id_value) in slots_mut.iter() {
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
                if let Some(effects) = item_metadata.get("effects").and_then(JsonValue::as_object) {
                    for (stat, delta) in effects {
                        if let Some(d) = delta.as_f64() {
                            *total_effects.entry(stat.clone()).or_insert(0.0) += d;
                        }
                    }
                }
            }

            // Get base stats (before equipment effects)
            let mut new_stats = entity_stats.clone();
            for (stat, bonus) in total_effects {
                let base = new_stats
                    .get(&stat)
                    .and_then(JsonValue::as_f64)
                    .unwrap_or(0.0);
                new_stats[stat] = JsonValue::from(base + bonus);
            }
            let _ = world.set_component(eid, "Stats", new_stats);

            // Apply changes if any
            let _ = world.set_component(eid, "Equipment", new_equipment);
        }
    }
}
