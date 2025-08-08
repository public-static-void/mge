use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::json;

pub struct ProcessDeaths;
impl System for ProcessDeaths {
    fn name(&self) -> &'static str {
        "ProcessDeaths"
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        let mut to_process = Vec::new();

        // Collect entities with Health <= 0
        if let Some(healths) = world.components.get("Health") {
            for (&entity, value) in healths.iter() {
                if let Some(obj) = value.as_object()
                    && let Some(current) = obj.get("current")
                    && current.as_f64().unwrap_or(1.0) <= 0.0
                {
                    to_process.push(entity);
                }
            }
        }

        for entity in to_process {
            // Remove Health component
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

/// System: Processes decay for entities with a Decay component.
pub struct ProcessDecay;
impl System for ProcessDecay {
    fn name(&self) -> &'static str {
        "ProcessDecay"
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        let mut to_despawn_entities = Vec::new();
        if let Some(decays) = world.components.get_mut("Decay") {
            for (&entity, value) in decays.iter_mut() {
                if let Some(obj) = value.as_object_mut()
                    && let Some(time_remaining) = obj.get_mut("time_remaining")
                    && let Some(t) = time_remaining.as_u64()
                {
                    if t <= 1 {
                        to_despawn_entities.push(entity);
                    } else {
                        *time_remaining = json!(t - 1);
                    }
                }
            }
        }
        for entity in to_despawn_entities {
            world.despawn_entity(entity);
        }
    }
}
