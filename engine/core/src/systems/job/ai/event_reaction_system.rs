use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::Value as JsonValue;

pub struct AiEventReactionSystem;

impl System for AiEventReactionSystem {
    fn name(&self) -> &'static str {
        "AiEventReactionSystem"
    }

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        while let Some(intent) = world.ai_event_intents.pop_front() {
            if let Some(kind) = intent.get("kind").and_then(|v| v.as_str()) {
                // For every agent, enqueue a production job for the scarce resource if not already queued
                let production_jobs: Vec<u32> = world
                    .components
                    .get("Job")
                    .map(|map| {
                        map.iter()
                            .filter_map(|(&eid, job)| {
                                if job.get("job_type").and_then(|v| v.as_str())
                                    == Some("production")
                                    && job
                                        .get("resource_outputs")
                                        .and_then(|v| v.as_array())
                                        .is_some_and(|outputs| {
                                            outputs.iter().any(|output| {
                                                output.get("kind").and_then(|v| v.as_str())
                                                    == Some(kind)
                                            })
                                        })
                                {
                                    Some(eid)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let mut updates = Vec::new();
                for (&agent_id, agent) in
                    world.components.get("Agent").unwrap_or(&Default::default())
                {
                    let mut queue = agent
                        .get("job_queue")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    for job_eid in &production_jobs {
                        if !queue.iter().any(|v| v.as_u64() == Some(*job_eid as u64)) {
                            queue.push(JsonValue::from(*job_eid));
                        }
                    }
                    updates.push((agent_id, queue));
                }
                // --- Now apply updates mutably ---
                for (agent_id, queue) in updates {
                    if let Some(agent_entry) = world
                        .components
                        .get_mut("Agent")
                        .and_then(|m| m.get_mut(&agent_id))
                    {
                        agent_entry["job_queue"] = JsonValue::from(queue);
                    }
                }
            }
        }
    }
}
