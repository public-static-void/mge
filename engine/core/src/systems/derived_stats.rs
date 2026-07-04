use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::{Map, Value as JsonValue};

/// DerivedStatsSystem computes secondary stats from primary Stats.
///
/// Formulas (per spec R015):
///   DerivedStats.MaxHP        = 100 + (stats.constitution or 0) * 10
///   DerivedStats.MeleeDamage  = 1.0 + (stats.strength or 0) * 0.5
///   DerivedStats.CritChance   = 0.05 + (stats.intelligence or 0) * 0.005
///
/// Runs after StatCalculationSystem (position 5 in execution order).
pub struct DerivedStatsSystem;

impl System for DerivedStatsSystem {
    fn name(&self) -> &'static str {
        "DerivedStatsSystem"
    }

    fn run(&mut self, world: &mut World) {
        for eid in world.get_entities_with_component("Stats") {
            let Some(stats) = world.get_component(eid, "Stats").cloned() else {
                continue;
            };

            let constitution = stats
                .get("constitution")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let strength = stats
                .get("strength")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let intelligence = stats
                .get("intelligence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let mut derived = Map::new();
            derived.insert(
                "MaxHP".to_string(),
                JsonValue::from(100.0 + constitution * 10.0),
            );
            derived.insert(
                "MeleeDamage".to_string(),
                JsonValue::from(1.0 + strength * 0.5),
            );
            derived.insert(
                "CritChance".to_string(),
                JsonValue::from(0.05 + intelligence * 0.005),
            );

            let _ = world.set_component(eid, "DerivedStats", JsonValue::Object(derived));
        }
    }
}
