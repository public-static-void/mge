use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::{Map, Value as JsonValue};

/// System for calculating stats from BaseStats and EquipmentEffects.
///
/// Single source of truth for stat computation:
///   Stats[k] = (BaseStats[k] || 0) + (EquipmentEffects[k] || 0)
///
/// Edge cases handled:
/// - No BaseStats component: entity is skipped (filtered by component query)
/// - Null BaseStats values: treated as 0 via unwrap_or(0.0)
/// - Key in EquipmentEffects but not in BaseStats: added to Stats with only the effect value
/// - No EquipmentEffects component: treated as empty, only BaseStats contribute
pub struct StatCalculationSystem;

impl System for StatCalculationSystem {
    fn name(&self) -> &'static str {
        "StatCalculationSystem"
    }

    fn run(&mut self, world: &mut World) {
        for eid in world.get_entities_with_component("BaseStats") {
            let Some(mut result) = world.get_component(eid, "BaseStats").cloned() else {
                continue;
            };
            // Default to empty object if no EquipmentEffects component
            let default_effects = JsonValue::Object(Map::new());
            let effects = world
                .get_component(eid, "EquipmentEffects")
                .unwrap_or(&default_effects);

            // Sum BaseStats and EquipmentEffects: result[k] = base[k] + effect[k]
            if let Some(effects_obj) = effects.as_object() {
                for (k, v) in effects_obj {
                    let base_val = result.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let delta = v.as_f64().unwrap_or(0.0);
                    result[k] = JsonValue::from(base_val + delta);
                }
            }
            let _ = world.set_component(eid, "Stats", result);
        }
    }
}
