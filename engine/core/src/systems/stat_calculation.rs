use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::{Map, Value as JsonValue};

/// System for calculating stats
pub struct StatCalculationSystem;

impl System for StatCalculationSystem {
    fn name(&self) -> &'static str {
        "StatCalculationSystem"
    }

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        for eid in world.get_entities_with_component("BaseStats") {
            let base = world.get_component(eid, "BaseStats").unwrap();
            let default_effects = JsonValue::Object(Map::new());
            let effects = world
                .get_component(eid, "EquipmentEffects")
                .unwrap_or(&default_effects);

            let mut result = base.clone();

            if let Some(effects_obj) = effects.as_object() {
                for (k, v) in effects_obj {
                    let base_val = result.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let delta = v.as_f64().unwrap_or(0.0);
                    result[k] = JsonValue::from(base_val + delta);
                }
            }
            world.set_component(eid, "Stats", result).unwrap();
        }
    }
}
