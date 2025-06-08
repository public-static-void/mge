use crate::ecs::world::World;
use crate::systems::job_board::JobBoard;
use serde_json::Value as JsonValue;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

// Global thread-safe buffer for AI event intents.
// (You can move this to a module if you prefer.)
lazy_static::lazy_static! {
    pub static ref AI_EVENT_INTENT_BUFFER: Arc<Mutex<VecDeque<JsonValue>>> = Arc::new(Mutex::new(VecDeque::new()));
}

fn compute_job_utility(agent: &JsonValue, job: &JsonValue, world: &World) -> f64 {
    let job_type = job.get("job_type").and_then(|v| v.as_str()).unwrap_or("");
    let empty = serde_json::Map::new();
    let skills = agent
        .get("skills")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty);
    let preferences = agent
        .get("preferences")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty);
    let skill = skills.get(job_type).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let pref = preferences
        .get(job_type)
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let mut resource_bonus = 0.0;
    if let Some(outputs) = job.get("resource_outputs").and_then(|v| v.as_array()) {
        for output in outputs {
            if let (Some(kind), Some(amount)) = (
                output.get("kind").and_then(|v| v.as_str()),
                output.get("amount").and_then(|v| v.as_i64()),
            ) {
                let scarcity = world.get_global_resource_scarcity(kind);
                resource_bonus += scarcity * amount as f64;
            }
        }
    }

    skill + pref + resource_bonus
}

pub fn assign_jobs(world: &mut World, job_board: &mut JobBoard) {
    let agent_ids: Vec<u32> = world
        .components
        .get("Agent")
        .map(|map| map.keys().copied().collect())
        .unwrap_or_default();

    let mut assignments: Vec<(u32, u32)> = Vec::new();
    let mut jobs_to_remove: Vec<u32> = Vec::new();

    for agent_id in &agent_ids {
        let agent = match world.components.get("Agent").and_then(|m| m.get(agent_id)) {
            Some(agent) => agent,
            None => continue,
        };
        let agent_state = agent
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("idle");

        if agent_state != "idle" && agent_state != "working" {
            continue;
        }

        let mut agent_queue = agent
            .get("job_queue")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if agent.get("current_job").is_none() && !agent_queue.is_empty() {
            let next_job_eid = agent_queue.remove(0).as_u64().unwrap() as u32;
            if let Some(job) = world.get_component(next_job_eid, "Job") {
                if job.get("status").and_then(|v| v.as_str()) == Some("pending") {
                    assignments.push((*agent_id, next_job_eid));
                    jobs_to_remove.push(next_job_eid);
                    if let Some(agent_entry) = world
                        .components
                        .get_mut("Agent")
                        .and_then(|m| m.get_mut(agent_id))
                    {
                        agent_entry["job_queue"] = JsonValue::from(agent_queue);
                    }
                    continue;
                }
            }
        }

        job_board.update(world);

        let mut best_job = None;
        let mut best_utility = f64::MIN;
        let mut best_priority = None;
        for &job_eid in &job_board.jobs {
            let job = match world.get_component(job_eid, "Job") {
                Some(j) => j,
                None => continue,
            };
            let priority = job.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
            let utility = compute_job_utility(agent, job, world);
            if utility > best_utility || (utility == best_utility && Some(priority) > best_priority)
            {
                best_utility = utility;
                best_priority = Some(priority);
                best_job = Some((job_eid, priority));
            }
        }

        if let Some((job_eid, _priority)) = best_job {
            assignments.push((*agent_id, job_eid));
            jobs_to_remove.push(job_eid);
        }
    }

    for (agent_id, job_eid) in assignments {
        if let Some(mut job) = world.get_component(job_eid, "Job").cloned() {
            job["assigned_to"] = JsonValue::from(agent_id);
            world.set_component(job_eid, "Job", job).unwrap();
        }
        if let Some(agent_entry) = world
            .components
            .get_mut("Agent")
            .and_then(|m| m.get_mut(&agent_id))
        {
            agent_entry["current_job"] = JsonValue::from(job_eid);
            agent_entry["state"] = JsonValue::from("working");
        }
    }
    job_board.jobs.retain(|eid| !jobs_to_remove.contains(eid));
}

pub fn setup_ai_event_subscriptions(world: &mut World) {
    use crate::ecs::event::EventBus;
    let registry = &mut world.event_buses;

    let bus = registry
        .get_event_bus::<serde_json::Value>("resource_shortage")
        .unwrap_or_else(|| {
            let new_bus = Arc::new(Mutex::new(EventBus::<serde_json::Value>::default()));
            registry.register_event_bus("resource_shortage".to_string(), new_bus.clone());
            new_bus
        });

    let buffer = AI_EVENT_INTENT_BUFFER.clone();
    let owner = Arc::new(());

    bus.lock().unwrap().subscribe_weak(&owner, move |event| {
        let mut queue = buffer.lock().unwrap();
        queue.push_back(event.clone());
    });
}
