use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// How many ticks it takes for a job's priority to increase by 1.
const AGING_FACTOR: u64 = 10;

/// The amount to boost priority for jobs needing a resource in shortage.
const SHORTAGE_PRIORITY_BOOST: i64 = 100;

/// System to update effective_priority for all jobs based on aging and world state.
#[derive(Default)]
pub struct JobPriorityAgingSystem;

impl JobPriorityAgingSystem {
    pub fn new() -> Self {
        Self
    }

    /// Updates effective_priority for all jobs.
    /// - effective_priority = priority + ((current_tick - creation_tick) / AGING_FACTOR)
    /// - If a resource shortage event is present, jobs needing that resource get a boost.
    pub fn run(&mut self, world: &mut World, current_tick: u64) {
        // Drain all resource_shortage events for this tick
        let shortage_events = world.drain_events::<serde_json::Value>("resource_shortage");
        let mut shortage_kinds = Vec::new();
        for event in &shortage_events {
            if let Some(kind) = event.get("kind").and_then(|v| v.as_str()) {
                shortage_kinds.push(kind.to_string());
            }
        }

        let job_eids = world.get_entities_with_component("Job");
        for &eid in &job_eids {
            let job = match world.get_component(eid, "Job") {
                Some(j) => j.clone(),
                None => continue,
            };
            let base_priority = job.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
            let creation_tick = job
                .get("creation_tick")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let aging_bonus = ((current_tick - creation_tick) / AGING_FACTOR) as i64;

            // Dynamic boost if any shortage matches this job's requirements
            let mut dynamic_boost = 0;
            if !shortage_kinds.is_empty() {
                if let Some(reqs) = job.get("resource_requirements").and_then(|v| v.as_array()) {
                    for req in reqs {
                        if let Some(kind) = req.get("kind").and_then(|v| v.as_str()) {
                            if shortage_kinds.iter().any(|k| k == kind) {
                                dynamic_boost = SHORTAGE_PRIORITY_BOOST;
                                break;
                            }
                        }
                    }
                }
            }

            let effective_priority = base_priority + aging_bonus + dynamic_boost;

            let mut job = job.clone();
            job["effective_priority"] = JsonValue::from(effective_priority);
            world.set_component(eid, "Job", job).unwrap();
        }
    }
}
