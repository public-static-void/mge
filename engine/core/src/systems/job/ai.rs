use crate::ecs::world::World;
use crate::systems::job_board::JobBoard;
use serde_json::Value as JsonValue;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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

    let mut jobs_to_remove: Vec<u32> = Vec::new();

    for agent_id in &agent_ids {
        let (agent_state, agent_queue, has_current_job) = {
            let agent = match world.components.get("Agent").and_then(|m| m.get(agent_id)) {
                Some(agent) => agent,
                None => continue,
            };
            let agent_state = agent
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("idle")
                .to_string();
            let agent_queue = agent
                .get("job_queue")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let has_current_job = agent.get("current_job").is_some();
            (agent_state, agent_queue, has_current_job)
        };

        // Only assign jobs to idle agents
        if agent_state != "idle" {
            continue;
        }

        let mut assigned_job = None;
        let mut new_queue = agent_queue.clone();
        if !has_current_job {
            while !new_queue.is_empty() {
                let next_job_eid = match new_queue[0].as_u64() {
                    Some(id) => id as u32,
                    None => {
                        new_queue.remove(0);
                        continue;
                    }
                };
                if let Some(job) = world.get_component(next_job_eid, "Job") {
                    if job.get("status").and_then(|v| v.as_str()) == Some("pending") {
                        assigned_job = Some(next_job_eid);
                        jobs_to_remove.push(next_job_eid);
                        new_queue.remove(0);
                        break;
                    } else {
                        new_queue.remove(0);
                    }
                } else {
                    new_queue.remove(0);
                }
            }
        }

        if assigned_job.is_none() && !has_current_job {
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
                let agent = world
                    .components
                    .get("Agent")
                    .and_then(|m| m.get(agent_id))
                    .unwrap();
                let utility = compute_job_utility(agent, job, world);
                if utility > best_utility
                    || (utility == best_utility && Some(priority) > best_priority)
                {
                    best_utility = utility;
                    best_priority = Some(priority);
                    best_job = Some((job_eid, priority));
                }
            }

            if let Some((job_eid, _priority)) = best_job {
                assigned_job = Some(job_eid);
                jobs_to_remove.push(job_eid);
            }
        }

        // Now do all mutations
        if let Some(job_eid) = assigned_job {
            if let Some(mut job) = world.get_component(job_eid, "Job").cloned() {
                job["assigned_to"] = serde_json::json!(*agent_id);
                world.set_component(job_eid, "Job", job).unwrap();
            }
            if let Some(agent_entry) = world.components.get("Agent").and_then(|m| m.get(agent_id)) {
                let mut agent_obj = agent_entry.clone();
                agent_obj["current_job"] = serde_json::json!(job_eid);
                agent_obj["state"] = serde_json::json!("working");
                agent_obj["job_queue"] = serde_json::json!(new_queue);
                world.set_component(*agent_id, "Agent", agent_obj).unwrap();
            }
        } else if !has_current_job {
            if let Some(agent_entry) = world.components.get("Agent").and_then(|m| m.get(agent_id)) {
                let mut agent_obj = agent_entry.clone();
                agent_obj["job_queue"] = serde_json::json!(new_queue);
                world.set_component(*agent_id, "Agent", agent_obj).unwrap();
            }
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
