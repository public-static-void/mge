use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::material::resolve_item_conductivity;
use serde_json::{Map, Value as JsonValue};
use std::collections::{HashMap, HashSet};

/// Weight of one covering item toward its body part's insulation.
pub fn insulation_contribution(base_insulation: f64, conductivity: f64) -> f64 {
    base_insulation * (1.0 - conductivity)
}

/// Equipment effect aggregation system
/// Aggregates the effects of all items in the same slot.
/// Sole writer of per-part `Body` insulation: each part's insulation is the
/// conductivity-weighted sum of its covering items, clamped to `[0.0, 1.0]`.
/// `TemperatureSystem` reads insulation and never writes it.
pub struct EquipmentEffectAggregationSystem;

impl System for EquipmentEffectAggregationSystem {
    fn name(&self) -> &'static str {
        "EquipmentEffectAggregationSystem"
    }

    fn run(&mut self, world: &mut World) {
        for eid in world.get_entities_with_component("Equipment") {
            let equipment = match world.get_component(eid, "Equipment") {
                Some(e) => e,
                None => continue,
            };
            let slots = match equipment.get("slots").and_then(|s| s.as_object()) {
                Some(s) => s,
                None => continue,
            };

            let mut effects: HashMap<String, f64> = HashMap::new();

            for (_slot, item_id_value) in slots {
                let item_id = match item_id_value.as_str() {
                    Some(id) => id,
                    None => continue,
                };
                if let Some(item) = world.item_registry.get_item(item_id)
                    && let Some(effects_obj) = item.get("effects").and_then(|v| v.as_object())
                {
                    for (k, v) in effects_obj {
                        let delta = v.as_f64().unwrap_or(0.0);
                        *effects.entry(k.clone()).or_insert(0.0) += delta;
                    }
                }
            }

            // Write the aggregated effects to EquipmentEffects
            let mut effect_map = Map::new();
            for (k, v) in effects {
                effect_map.insert(k, JsonValue::from(v));
            }
            let _ = world.set_component(eid, "EquipmentEffects", JsonValue::Object(effect_map));
        }

        write_body_insulation(world);
    }
}

/// Recompute every `Body` part's insulation from its covering items.
///
/// Covering set per part = item ids in the part's `equipped` array (synced by
/// `BodyEquipmentSyncSystem`) plus the `Equipment` slot entry naming that
/// part when it is not yet synced. Either source alone suffices, so the
/// result is independent of whether sync ran before or after this system.
fn write_body_insulation(world: &mut World) {
    let mut entities = world.get_entities_with_component("Body");
    entities.sort_unstable();
    for entity in entities {
        let mut body = match world.get_component(entity, "Body").cloned() {
            Some(b) => b,
            None => continue,
        };
        let slots: HashMap<String, String> = world
            .get_component(entity, "Equipment")
            .and_then(|e| e.get("slots"))
            .and_then(|s| s.as_object())
            .map(|slots| {
                slots
                    .iter()
                    .filter_map(|(slot, id)| id.as_str().map(|id| (slot.clone(), id.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) else {
            continue;
        };
        for part in parts.iter_mut() {
            write_part_insulation(world, entity, &slots, part);
        }
        let _ = world.set_component(entity, "Body", body);
    }
}

fn write_part_insulation(
    world: &World,
    entity: u32,
    slots: &HashMap<String, String>,
    part: &mut JsonValue,
) {
    let name = part
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let mut covering: Vec<String> = part
        .get("equipped")
        .and_then(|v| v.as_array())
        .map(|ids| {
            ids.iter()
                .filter_map(|id| id.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if !name.is_empty()
        && let Some(item_id) = slots.get(&name)
        && !covering.contains(item_id)
    {
        covering.push(item_id.clone());
    }

    let mut seen = HashSet::new();
    let mut total = 0.0;
    for item_id in &covering {
        if !seen.insert(item_id) {
            continue;
        }
        let Some(item) = world.item_registry.get_item(item_id) else {
            continue;
        };
        let base = item
            .get("effects")
            .and_then(|e| e.get("insulation"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        total += insulation_contribution(base, resolve_item_conductivity(world, entity, item));
    }
    part["insulation"] = JsonValue::from(total.clamp(0.0, 1.0));

    if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut()) {
        for child in children.iter_mut() {
            write_part_insulation(world, entity, slots, child);
        }
    }
}
