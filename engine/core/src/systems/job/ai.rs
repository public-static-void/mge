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
        let (mut agent_state, agent_queue, mut has_current_job, mut current_job_eid) = {
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
            let current_job = agent
                .get("current_job")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let has_current_job = current_job.is_some();
            (agent_state, agent_queue, has_current_job, current_job)
        };

        // --- Abandon blocked job logic ---
        if let Some(job_eid) = current_job_eid {
            if let Some(job) = world.get_component(job_eid, "Job") {
                if job
                    .get("blocked")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    // Unassign agent from job
                    let mut job = job.clone();
                    job.as_object_mut().unwrap().remove("assigned_to");
                    world.set_component(job_eid, "Job", job).unwrap();

                    // Unassign job from agent
                    if let Some(agent_entry) =
                        world.components.get("Agent").and_then(|m| m.get(agent_id))
                    {
                        let mut agent_obj = agent_entry.clone();
                        agent_obj.as_object_mut().unwrap().remove("current_job");
                        agent_obj["state"] = serde_json::json!("idle");
                        world.set_component(*agent_id, "Agent", agent_obj).unwrap();
                    }
                    // After abandoning, treat as idle for this tick
                    agent_state = "idle".to_string();
                    has_current_job = false;
                    current_job_eid = None;
                }
            }
        }

        // --- Preemption logic ---
        if agent_state == "working" && has_current_job {
            let current_job_eid = current_job_eid.unwrap();
            let current_job = world.get_component(current_job_eid, "Job");
            let current_priority = current_job
                .and_then(|job| job.get("priority").and_then(|v| v.as_i64()))
                .unwrap_or(0);
            let agent = world
                .components
                .get("Agent")
                .and_then(|m| m.get(agent_id))
                .unwrap();

            job_board.update(world);

            // Find the best available job for this agent
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
                // Only consider jobs with higher priority than current
                if (priority > current_priority)
                    && (utility > best_utility
                        || (utility == best_utility && Some(priority) > best_priority))
                {
                    best_utility = utility;
                    best_priority = Some(priority);
                    best_job = Some((job_eid, priority));
                }
            }

            if let Some((new_job_eid, _priority)) = best_job {
                // Unassign agent from current job
                if let Some(mut job) = world.get_component(current_job_eid, "Job").cloned() {
                    job.as_object_mut().unwrap().remove("assigned_to");
                    job["status"] = serde_json::json!("pending");
                    world.set_component(current_job_eid, "Job", job).unwrap();
                }
                // Push preempted job back to agent's queue
                if let Some(agent_entry) =
                    world.components.get("Agent").and_then(|m| m.get(agent_id))
                {
                    let mut agent_obj = agent_entry.clone();
                    // Insert at front or back depending on desired behavior (here: front)
                    let mut queue = agent_obj
                        .get("job_queue")
                        .and_then(|v| v.as_array().cloned())
                        .unwrap_or_default();
                    queue.insert(0, serde_json::json!(current_job_eid));
                    agent_obj["job_queue"] = serde_json::json!(queue);
                    world.set_component(*agent_id, "Agent", agent_obj).unwrap();
                }
                // Assign agent to new job
                if let Some(mut job) = world.get_component(new_job_eid, "Job").cloned() {
                    job["assigned_to"] = serde_json::json!(*agent_id);
                    world.set_component(new_job_eid, "Job", job).unwrap();
                }
                // Update agent
                if let Some(agent_entry) =
                    world.components.get("Agent").and_then(|m| m.get(agent_id))
                {
                    let mut agent_obj = agent_entry.clone();
                    agent_obj["current_job"] = serde_json::json!(new_job_eid);
                    agent_obj["state"] = serde_json::json!("working");
                    world.set_component(*agent_id, "Agent", agent_obj).unwrap();
                }
                jobs_to_remove.push(new_job_eid);
                continue; // Preemption done, skip to next agent
            }
        }

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
