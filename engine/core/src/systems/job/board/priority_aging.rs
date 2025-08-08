use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// How many ticks it takes for a job's priority to increase by 1.
const AGING_FACTOR: u64 = 10;

/// The amount to boost priority for jobs needing a resource in shortage.
const SHORTAGE_PRIORITY_BOOST: i64 = 100;

/// System to update priorities for all jobs based on aging and world state.
#[derive(Default)]
pub struct JobPriorityAgingSystem;

impl JobPriorityAgingSystem {
    pub fn new() -> Self {
        Self
    }

    /// Collects all resource kinds in shortage from events.
    pub fn get_shortage_kinds(world: &mut World) -> Vec<String> {
        let shortage_events = world.drain_events::<serde_json::Value>("resource_shortage");
        shortage_events
            .iter()
            .filter_map(|event| event.get("kind").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .collect()
    }

    /// Computes the effective priority for a job in the given world at the given tick.
    /// - effective_priority = priority + ((current_tick - creation_tick) / AGING_FACTOR)
    /// - If a resource shortage applies, jobs needing that resource get a boost.
    pub fn compute_effective_priority(
        job: &JsonValue,
        current_tick: u64,
        shortage_kinds: &[String],
    ) -> i64 {
        let base_priority = job.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
        let creation_tick = job
            .get("creation_tick")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let aging_bonus = ((current_tick - creation_tick) / AGING_FACTOR) as i64;

        let mut dynamic_boost = 0;
        if !shortage_kinds.is_empty()
            && let Some(reqs) = job.get("resource_requirements").and_then(|v| v.as_array())
        {
            for req in reqs {
                if let Some(kind) = req.get("kind").and_then(|v| v.as_str())
                    && shortage_kinds.iter().any(|k| k == kind)
                {
                    dynamic_boost = SHORTAGE_PRIORITY_BOOST;
                    break;
                }
            }
        }
        base_priority + aging_bonus + dynamic_boost
    }

    /// This function does nothing now (kept for compatibility).
    pub fn run(&mut self, _world: &mut World, _current_tick: u64) {
        // No-op: effective priority is now computed on-demand, not stored.
        // This removes schema pollution and allows external callers to query as needed.
    }
}
