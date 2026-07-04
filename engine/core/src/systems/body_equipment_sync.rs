use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::{Value as JsonValue, json};

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

/// Syncs Body and Equipment components
/// Handles body↔equipment bidirectional sync and body part health → auto-unequip logic.
/// Does NOT write to Stats (responsibility of downstream systems).
pub struct BodyEquipmentSyncSystem;

impl System for BodyEquipmentSyncSystem {
    fn name(&self) -> &'static str {
        "BodyEquipmentSyncSystem"
    }

    fn run(&mut self, world: &mut World) {
        for eid in world.get_entities_with_component("Body") {
            let mut body = match world.get_component(eid, "Body").cloned() {
                Some(b) => b,
                None => continue,
            };

            let mut equipment = match world.get_component(eid, "Equipment").cloned() {
                Some(e) => e,
                None => continue,
            };

            let Some(mut slots) = equipment.get("slots").and_then(|v| v.as_object()).cloned()
            else {
                continue;
            };

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
                        let Some(equipped) =
                            part.get_mut("equipped").and_then(|v| v.as_array_mut())
                        else {
                            continue;
                        };
                        equipped.clear();
                        if status == "healthy"
                            && let Some(item_id) = item_id_val.as_str()
                        {
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

            // Step 4: Write back updated Equipment and Body components (no stat writes)
            equipment["slots"] = JsonValue::Object(slots);
            let _ = world.set_component(eid, "Equipment", equipment.clone());
            let _ = world.set_component(eid, "Body", body.clone());
        }
    }
}
