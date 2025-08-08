use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::{Value as JsonValue, json};
use std::collections::HashMap;

/// Recursively ensure every part (and its children) has "equipped" and "children" as arrays.
/// Also ensures the top-level "parts" field on the body is always an array.
fn ensure_body_part_arrays(body: &mut JsonValue) {
    // Ensure top-level "parts" is always an array
    if !body.get("parts").is_some_and(|v| v.is_array()) {
        body["parts"] = json!([]);
    }
    if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
        for part in parts {
            ensure_part_arrays_recursive(part);
        }
    }
}

fn ensure_part_arrays_recursive(part: &mut JsonValue) {
    if !part.get("equipped").is_some_and(|v| v.is_array()) {
        part["equipped"] = json!([]);
    }
    if !part.get("children").is_some_and(|v| v.is_array()) {
        part["children"] = json!([]);
    }
    if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut()) {
        for child in children {
            ensure_part_arrays_recursive(child);
        }
    }
}

fn find_part_mut<'a>(part: &'a mut JsonValue, name: &str) -> Option<&'a mut JsonValue> {
    if part.get("name").and_then(|n| n.as_str()) == Some(name) {
        return Some(part);
    }
    if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut()) {
        for child in children {
            if let Some(found) = find_part_mut(child, name) {
                return Some(found);
            }
        }
    }
    None
}

fn collect_equipped_items(part: &JsonValue, equipped: &mut Vec<String>) {
    if let Some(eq) = part.get("equipped").and_then(|v| v.as_array()) {
        for item in eq {
            if let Some(id) = item.as_str() {
                equipped.push(id.to_string());
            }
        }
    }
    if let Some(children) = part.get("children").and_then(|v| v.as_array()) {
        for child in children {
            collect_equipped_items(child, equipped);
        }
    }
}

fn clear_equipped_on_unhealthy_parts(part: &mut JsonValue) {
    let status = part
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("healthy");
    if status != "healthy" {
        if let Some(eq) = part.get_mut("equipped").and_then(|v| v.as_array_mut()) {
            eq.clear();
        }
        // Ensure equipped is still present as an array
        if !part.get("equipped").is_some_and(|v| v.is_array()) {
            part["equipped"] = json!([]);
        }
    }
    if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut()) {
        for child in children {
            clear_equipped_on_unhealthy_parts(child);
        }
    }
}

fn find_part<'a>(part: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    if part.get("name").and_then(|n| n.as_str()) == Some(name) {
        return Some(part);
    }
    if let Some(children) = part.get("children").and_then(|v| v.as_array()) {
        for child in children {
            if let Some(found) = find_part(child, name) {
                return Some(found);
            }
        }
    }
    None
}

pub struct BodyEquipmentSyncSystem;

impl System for BodyEquipmentSyncSystem {
    fn name(&self) -> &'static str {
        "BodyEquipmentSyncSystem"
    }

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        for eid in world.get_entities_with_component("Body") {
            let mut body = match world.get_component(eid, "Body").cloned() {
                Some(b) => b,
                None => continue,
            };

            let mut equipment = match world.get_component(eid, "Equipment").cloned() {
                Some(e) => e,
                None => continue,
            };

            let slots = equipment.get("slots").and_then(|v| v.as_object()).cloned();
            if slots.is_none() {
                continue;
            }
            let mut slots = slots.unwrap();

            // Step 0: Always clear equipped on unhealthy parts
            if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
                for part in parts {
                    clear_equipped_on_unhealthy_parts(part);
                }
            }

            // Step 1: Equipment -> Body (only equip if healthy)
            if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
                for (slot_name, item_id_val) in slots.iter_mut() {
                    let part = parts.iter_mut().find_map(|p| find_part_mut(p, slot_name));
                    if let Some(part) = part {
                        let status = part
                            .get("status")
                            .and_then(|s| s.as_str())
                            .unwrap_or("healthy")
                            .to_string();
                        // Always ensure equipped is present as an array
                        if !part.get("equipped").is_some_and(|v| v.is_array()) {
                            part["equipped"] = json!([]);
                        }
                        let equipped = part
                            .get_mut("equipped")
                            .and_then(|v| v.as_array_mut())
                            .unwrap();
                        // Always clear equipped first
                        equipped.clear();
                        if status == "healthy" && !item_id_val.is_null() {
                            let item_id = item_id_val.as_str().unwrap();
                            equipped.push(json!(item_id));
                        }
                        // If not healthy, forcibly clear the Equipment slot too
                        if status != "healthy" {
                            *item_id_val = JsonValue::Null;
                        }
                    }
                }
            }

            // Step 2: Body -> Equipment (always update slot from equipped)
            let slot_names: Vec<String> = slots.keys().cloned().collect();
            let mut slot_infos = Vec::new();
            if let Some(parts) = body.get("parts").and_then(|v| v.as_array()) {
                for slot_name in &slot_names {
                    if let Some(part) = parts.iter().find_map(|p| find_part(p, slot_name)) {
                        let status = part
                            .get("status")
                            .and_then(|s| s.as_str())
                            .unwrap_or("healthy")
                            .to_string();
                        let eq = part
                            .get("equipped")
                            .and_then(|v| v.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        let item_id = part
                            .get("equipped")
                            .and_then(|v| v.as_array())
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        slot_infos.push((slot_name.clone(), status, eq, item_id));
                    }
                }
            }
            for (slot_name, status, eq_len, item_id_opt) in slot_infos {
                if status != "healthy" || eq_len == 0 {
                    slots.insert(slot_name.clone(), JsonValue::Null);
                } else if let Some(item_id) = item_id_opt {
                    slots.insert(slot_name.clone(), JsonValue::String(item_id));
                }
            }

            // Step 3: Recursively ensure all arrays are present and schema-compliant
            ensure_body_part_arrays(&mut body);

            // Step 4: Write back updated Equipment and Body components
            equipment["slots"] = JsonValue::Object(slots);
            let _ = world.set_component(eid, "Equipment", equipment.clone());
            let _ = world.set_component(eid, "Body", body.clone());

            // Step 5: Aggregate effects from all equipped items on body parts
            let mut equipped_items = Vec::new();
            if let Some(parts) = body.get("parts").and_then(|v| v.as_array()) {
                for part in parts {
                    collect_equipped_items(part, &mut equipped_items);
                }
            }

            let mut total_effects: HashMap<String, f64> = HashMap::new();
            for item_id in equipped_items {
                for item_eid in world.get_entities_with_component("Item") {
                    if let Some(item_comp) = world.get_component(item_eid, "Item")
                        && item_comp.get("id").and_then(|v| v.as_str()) == Some(&item_id)
                            && let Some(effects) =
                                item_comp.get("effects").and_then(|v| v.as_object())
                            {
                                for (stat, delta) in effects {
                                    if let Some(d) = delta.as_f64() {
                                        *total_effects.entry(stat.clone()).or_insert(0.0) += d;
                                    }
                                }
                            }
                }
            }

            // Apply aggregated effects to Stats component
            let mut stats = world
                .get_component(eid, "Stats")
                .cloned()
                .unwrap_or_else(|| json!({}));
            for (stat, bonus) in total_effects {
                let base = stats.get(&stat).and_then(|v| v.as_f64()).unwrap_or(0.0);
                stats[stat] = json!(base + bonus);
            }
            let _ = world.set_component(eid, "Stats", stats);
        }
    }
}
