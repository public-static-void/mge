use crate::ecs::system::System;
use crate::ecs::world::World;

/// System: Processes reputation decay for entities with a Reputation component.
/// Each tick, reputation values decay toward zero by the decay_rate.
pub struct FactionReputationSystem;

impl System for FactionReputationSystem {
    fn name(&self) -> &'static str {
        "FactionReputationSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    fn run(&mut self, world: &mut World) {
        let mut to_update: Vec<(u32, String, i64)> = Vec::new();

        // Collect decay operations: for each entity with Reputation,
        // for each entry in values, apply decay toward 0.
        if let Some(reputations) = world.components.get("Reputation") {
            for (&entity, value) in reputations.iter() {
                let decay_rate = value
                    .get("decay_rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                // Skip if decay_rate is 0.0 or effectively zero
                if decay_rate.abs() < f64::EPSILON {
                    continue;
                }

                if let Some(values) = value.get("values").and_then(|v| v.as_object()) {
                    for (faction_id, score_val) in values {
                        if let Some(current) = score_val.as_i64() {
                            let decay = decay_rate as i64;
                            let new_value = if current > 0 {
                                (current - decay).max(0)
                            } else if current < 0 {
                                (current + decay).min(0)
                            } else {
                                continue;
                            };
                            // Clamp to [-100, 100]
                            let clamped = new_value.clamp(-100, 100);
                            if clamped != current {
                                to_update.push((entity, faction_id.clone(), clamped));
                            }
                        }
                    }
                }
            }
        }

        // Apply collected updates
        for (entity, faction_id, new_value) in to_update {
            if let Some(reputations) = world.components.get_mut("Reputation") {
                if let Some(value) = reputations.get_mut(&entity) {
                    if let Some(values) = value.get_mut("values") {
                        if let Some(obj) = values.as_object_mut() {
                            obj.insert(faction_id, serde_json::json!(new_value));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::registry::ComponentRegistry;
    use crate::ecs::schema::ComponentSchema;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    fn setup_world() -> World {
        let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
        {
            let mut reg = registry.lock().unwrap();
            let _ = reg.register_external_schema(ComponentSchema {
                name: "Faction".to_string(),
                schema: serde_json::from_str(include_str!(
                    "../../../assets/schemas/faction.json"
                ))
                .unwrap(),
                modes: vec![
                    "colony".to_string(),
                    "roguelike".to_string(),
                    "simulation".to_string(),
                ],
            });
            let _ = reg.register_external_schema(ComponentSchema {
                name: "Reputation".to_string(),
                schema: serde_json::from_str(include_str!(
                    "../../../assets/schemas/reputation.json"
                ))
                .unwrap(),
                modes: vec![
                    "colony".to_string(),
                    "roguelike".to_string(),
                    "simulation".to_string(),
                ],
            });
        }
        let mut world = World::new(registry);
        world.current_mode = "colony".to_string();
        world
    }

    #[test]
    fn test_decay_positive_toward_zero() {
        let mut world = setup_world();
        let entity = world.spawn_entity();
        world
            .set_component(
                entity,
                "Reputation",
                json!({
                    "values": { "goblins": 50 },
                    "decay_rate": 10.0,
                }),
            )
            .unwrap();

        let mut system = FactionReputationSystem;
        system.run(&mut world);

        let comp = world.get_component(entity, "Reputation").unwrap();
        let value = comp["values"]["goblins"].as_i64().unwrap();
        assert_eq!(value, 40);
    }

    #[test]
    fn test_decay_negative_toward_zero() {
        let mut world = setup_world();
        let entity = world.spawn_entity();
        world
            .set_component(
                entity,
                "Reputation",
                json!({
                    "values": { "goblins": -50 },
                    "decay_rate": 10.0,
                }),
            )
            .unwrap();

        let mut system = FactionReputationSystem;
        system.run(&mut world);

        let comp = world.get_component(entity, "Reputation").unwrap();
        let value = comp["values"]["goblins"].as_i64().unwrap();
        assert_eq!(value, -40);
    }

    #[test]
    fn test_decay_skips_zero_decay_rate() {
        let mut world = setup_world();
        let entity = world.spawn_entity();
        world
            .set_component(
                entity,
                "Reputation",
                json!({
                    "values": { "goblins": 50 },
                    "decay_rate": 0.0,
                }),
            )
            .unwrap();

        let mut system = FactionReputationSystem;
        system.run(&mut world);

        let comp = world.get_component(entity, "Reputation").unwrap();
        let value = comp["values"]["goblins"].as_i64().unwrap();
        assert_eq!(value, 50);
    }

    #[test]
    fn test_decay_does_not_cross_zero() {
        let mut world = setup_world();
        let entity = world.spawn_entity();
        world
            .set_component(
                entity,
                "Reputation",
                json!({
                    "values": { "goblins": 5 },
                    "decay_rate": 10.0,
                }),
            )
            .unwrap();

        let mut system = FactionReputationSystem;
        system.run(&mut world);

        let comp = world.get_component(entity, "Reputation").unwrap();
        let value = comp["values"]["goblins"].as_i64().unwrap();
        // Should decay to 0, not cross to negative
        assert_eq!(value, 0);
    }

    #[test]
    fn test_decay_does_not_exceed_bounds() {
        let mut world = setup_world();
        let entity = world.spawn_entity();
        world
            .set_component(
                entity,
                "Reputation",
                json!({
                    "values": { "goblins": 100 },
                    "decay_rate": 10.0,
                }),
            )
            .unwrap();

        let mut system = FactionReputationSystem;
        system.run(&mut world);

        let comp = world.get_component(entity, "Reputation").unwrap();
        let value = comp["values"]["goblins"].as_i64().unwrap();
        assert_eq!(value, 90);
    }

    #[test]
    fn test_system_name() {
        let system = FactionReputationSystem;
        assert_eq!(system.name(), "FactionReputationSystem");
    }

    #[test]
    fn test_system_dependencies() {
        let system = FactionReputationSystem;
        assert!(system.dependencies().is_empty());
    }
}
