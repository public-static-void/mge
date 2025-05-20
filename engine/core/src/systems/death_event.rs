use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::json;

struct DeathDetector;

impl System for DeathDetector {
    fn name(&self) -> &'static str {
        "DeathDetector"
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        // Phase 1: Collect dead entities
        let dead_entities: Vec<u32> = world
            .components
            .get("Health")
            .map(|healths| {
                healths
                    .iter()
                    .filter_map(|(&entity, value)| {
                        value
                            .as_object()
                            .and_then(|obj| obj.get("current"))
                            .and_then(|current| {
                                if current.as_f64().unwrap_or(1.0) <= 0.0 {
                                    Some(entity)
                                } else {
                                    None
                                }
                            })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Phase 2: Emit events
        for entity in dead_entities {
            world.emit_event("EntityDied", json!({ "entity": entity }));
        }
    }
}

struct DeathProcessor;

impl System for DeathProcessor {
    fn name(&self) -> &'static str {
        "DeathProcessor"
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        // Phase 1: Collect entity IDs from events
        let mut to_process = Vec::new();
        world.process_events("EntityDied", |payload| {
            if let Some(entity_val) = payload.get("entity") {
                if let Some(entity) = entity_val.as_u64() {
                    to_process.push(entity as u32);
                }
            }
        });

        // Phase 2: Mutate world
        for entity in to_process {
            if let Some(healths) = world.components.get_mut("Health") {
                healths.remove(&entity);
            }
            let _ = world.set_component(entity, "Corpse", json!({}));
            let _ = world.set_component(entity, "Decay", json!({ "time_remaining": 5 }));
        }
    }
}
