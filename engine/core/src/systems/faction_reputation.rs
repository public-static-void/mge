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
