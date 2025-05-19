use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;

pub struct EquipmentEffectAggregationSystem;

impl System for EquipmentEffectAggregationSystem {
    fn name(&self) -> &'static str {
        "EquipmentEffectAggregationSystem"
    }

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
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
                for item_eid in world.get_entities_with_component("Item") {
                    let item = match world.get_component(item_eid, "Item") {
                        Some(i) => i,
                        None => continue,
                    };
                    if item.get("id").and_then(|v| v.as_str()) != Some(item_id) {
                        continue;
                    }
                    if let Some(effects_obj) = item.get("effects").and_then(|v| v.as_object()) {
                        for (k, v) in effects_obj {
                            let delta = v.as_f64().unwrap_or(0.0);
                            *effects.entry(k.clone()).or_insert(0.0) += delta;
                        }
                    }
                }
            }

            // Write the aggregated effects to EquipmentEffects
            let mut effect_map = Map::new();
            for (k, v) in effects {
                effect_map.insert(k, JsonValue::from(v));
            }
            world
                .set_component(eid, "EquipmentEffects", JsonValue::Object(effect_map))
                .unwrap();
        }
    }
}
