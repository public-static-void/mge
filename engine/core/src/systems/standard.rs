use crate::ecs::system::System;
use crate::scripting::world::World;
use serde_json::json;

pub struct MoveAll {
    pub dx: f32,
    pub dy: f32,
}
impl System for MoveAll {
    fn name(&self) -> &'static str {
        "MoveAll"
    }
    fn run(&mut self, world: &mut World) {
        if let Some(positions) = world.components.get_mut("Position") {
            for (_entity, value) in positions.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(x) = obj.get_mut("x") {
                        if let Some(x_val) = x.as_f64() {
                            *x = serde_json::json!(x_val + self.dx as f64);
                        }
                    }
                    if let Some(y) = obj.get_mut("y") {
                        if let Some(y_val) = y.as_f64() {
                            *y = serde_json::json!(y_val + self.dy as f64);
                        }
                    }
                }
            }
        }
    }
}

pub struct ProcessDeaths;
impl System for ProcessDeaths {
    fn name(&self) -> &'static str {
        "ProcessDeaths"
    }
    fn run(&mut self, world: &mut World) {
        let mut to_process = Vec::new();

        // Collect entities with Health <= 0
        if let Some(healths) = world.components.get("Health") {
            for (&entity, value) in healths.iter() {
                if let Some(obj) = value.as_object() {
                    if let Some(current) = obj.get("current") {
                        if current.as_f64().unwrap_or(1.0) <= 0.0 {
                            to_process.push(entity);
                        }
                    }
                }
            }
        }

        for entity in to_process {
            // Remove Health component (if you want to simulate "dead" state)
            if let Some(healths) = world.components.get_mut("Health") {
                healths.remove(&entity);
            }

            // Add Corpse component
            let _ = world.set_component(entity, "Corpse", json!({}));

            // Add Decay component with default time_remaining (e.g., 5 ticks)
            let _ = world.set_component(entity, "Decay", json!({ "time_remaining": 5 }));
        }
    }
}

/// System: Damages all entities with a Health component by a given amount.
pub struct DamageAll {
    pub amount: f32,
}
impl System for DamageAll {
    fn name(&self) -> &'static str {
        "DamageAll"
    }
    fn run(&mut self, world: &mut World) {
        if let Some(healths) = world.components.get_mut("Health") {
            for (_entity, value) in healths.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(current) = obj.get_mut("current") {
                        if let Some(cur_val) = current.as_f64() {
                            let new_val = (cur_val - self.amount as f64).max(0.0);
                            *current = json!(new_val);
                        }
                    }
                }
            }
        }
    }
}

/// System: Processes decay for entities with a Decay component.
pub struct ProcessDecay;
impl System for ProcessDecay {
    fn name(&self) -> &'static str {
        "ProcessDecay"
    }
    fn run(&mut self, world: &mut World) {
        let mut to_despawn_entities = Vec::new();
        if let Some(decays) = world.components.get_mut("Decay") {
            for (&entity, value) in decays.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(time_remaining) = obj.get_mut("time_remaining") {
                        if let Some(t) = time_remaining.as_u64() {
                            if t <= 1 {
                                to_despawn_entities.push(entity);
                            } else {
                                *time_remaining = json!(t - 1);
                            }
                        }
                    }
                }
            }
        }
        for entity in to_despawn_entities {
            world.despawn_entity(entity);
        }
    }
}
