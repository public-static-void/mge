use crate::ecs::world::World;
use crate::systems::job::job_board::JobBoard;
use serde_json::Value as JsonValue;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    pub static ref AI_EVENT_INTENT_BUFFER: Arc<Mutex<VecDeque<JsonValue>>> = Arc::new(Mutex::new(VecDeque::new()));
}

/// Computes the utility score of a job for a given agent.
/// Takes into account skills, preferences, specializations, and resource bonuses.
fn compute_job_utility(agent: &JsonValue, job: &JsonValue, world: &World) -> f64 {
    let job_type = job.get("job_type").and_then(|v| v.as_str()).unwrap_or("");
    let job_category = job.get("category").and_then(|v| v.as_str()).unwrap_or("");
    let empty = serde_json::Map::new();
    let skills = agent
        .get("skills")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty);
    let preferences = agent
        .get("preferences")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty);
    let specializations = agent
        .get("specializations")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

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

    let specialization_bonus =
        if !job_category.is_empty() && specializations.contains(&job_category) {
            1000.0
        } else {
            0.0
        };

    skill + pref + resource_bonus + specialization_bonus
}

/// Assigns jobs to agents based on utility, priority, job queue, and specialization.
/// Prefers assigning jobs to agents whose specializations match the job's category.
/// If no such agent is available, will assign jobs to any idle agent as a fallback.
/// Handles job preemption, agent job queues, and blocked jobs.
/// Always sets unassigned fields to `null` (never removes them).
/// Always checks for value, not just presence.
/// Robust to schema changes and serialization quirks.
pub fn assign_jobs(
    world: &mut World,
    job_board: &mut JobBoard,
    current_tick: u64,
    shortage_kinds: &[String],
) {
    use std::collections::{HashMap, HashSet};

    let mut agent_ids: Vec<u32> = world
        .components
        .get("Agent")
        .map(|map| map.keys().copied().collect())
        .unwrap_or_default();
    agent_ids.sort();

    job_board.update(world, current_tick, shortage_kinds);

    let mut jobs_to_remove: Vec<u32> = Vec::new();
    let mut assigned_jobs: HashSet<u32> = HashSet::new();
    let mut preempted_jobs: HashMap<u32, u32> = HashMap::new(); // agent_id -> job_id

    // Always clean up any agent still holding a blocked job
    for &agent_id in &agent_ids {
        let agent_opt = world.components.get("Agent").and_then(|m| m.get(&agent_id));
        let current_job_eid = agent_opt.and_then(|agent| {
            agent.get("current_job").and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_u64().map(|v| v as u32)
                }
            })
        });
        if let (Some(agent), Some(job_eid)) = (agent_opt, current_job_eid) {
            let job_state_is_blocked = world
                .get_component(job_eid, "Job")
                .and_then(|job| job.get("state").and_then(|v| v.as_str()))
                .map(|s| s == "blocked")
                .unwrap_or(false);
            if job_state_is_blocked {
                let mut agent_obj = agent.clone();
                agent_obj["current_job"] = serde_json::Value::Null;
                agent_obj["state"] = serde_json::json!("idle");
                world.set_component(agent_id, "Agent", agent_obj).unwrap();
                if let Some(job) = world.get_component(job_eid, "Job").cloned() {
                    let mut job_obj = job;
                    job_obj["assigned_to"] = serde_json::Value::Null;
                    world.set_component(job_eid, "Job", job_obj).unwrap();
                }
            }
        }
    }

    // === First pass: assign jobs to agents with matching specialization ===
    for agent_id in &agent_ids {
        let (mut agent_state, agent_queue, mut has_current_job, mut current_job_eid) = {
            let agent = match world.components.get("Agent").and_then(|m| m.get(agent_id)) {
                Some(agent) => agent,
                None => {
                    continue;
                }
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
            let current_job = agent.get("current_job").and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_u64().map(|v| v as u32)
                }
            });
            let has_current_job = current_job.is_some();
            (agent_state, agent_queue, has_current_job, current_job)
        };

        let mut became_idle_this_tick = false;
        if let Some(job_eid) = current_job_eid {
            let job_state_is_blocked = world
                .get_component(job_eid, "Job")
                .and_then(|job| job.get("state").and_then(|v| v.as_str()))
                .map(|s| s == "blocked")
                .unwrap_or(false);
            if job_state_is_blocked {
                agent_state = "idle".to_string();
                has_current_job = false;
                current_job_eid = None;
                became_idle_this_tick = true;
            }
        }

        let mut preempted_this_tick = false;
        if agent_state == "working" && has_current_job {
            let current_job_eid_val = current_job_eid.unwrap();
            let current_job = world.get_component(current_job_eid_val, "Job");
            let current_priority = current_job
                .and_then(|job| job.get("priority").and_then(|v| v.as_i64()))
                .unwrap_or(0);
            let agent = world
                .components
                .get("Agent")
                .and_then(|m| m.get(agent_id))
                .unwrap();

            let mut best_job = None;
            let mut best_utility = f64::MIN;
            let mut best_priority = None;
            let specializations = agent
                .get("specializations")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();

            let all_job_ids: Vec<u32> = world
                .components
                .get("Job")
                .map(|map| map.keys().copied().collect())
                .unwrap_or_default();

            for &job_eid in &all_job_ids {
                if assigned_jobs.contains(&job_eid) {
                    continue;
                }
                let job = match world.get_component(job_eid, "Job") {
                    Some(j) => j,
                    None => continue,
                };

                let job_category = job.get("category").and_then(|v| v.as_str()).unwrap_or("");
                let is_specialist =
                    !job_category.is_empty() && specializations.contains(&job_category);

                let specialization_bonus = if is_specialist { 1000.0 } else { 0.0 };

                let priority = job.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
                let utility = compute_job_utility(agent, job, world) + specialization_bonus;

                let job_state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                let assigned = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0);

                if job_state != "pending"
                    || assigned == *agent_id as u64
                    || job_state == "blocked"
                    || job_state == "failed"
                    || job_state == "complete"
                    || job_state == "cancelled"
                    || job_state == "interrupted"
                {
                    continue;
                }

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
                if let Some(job) = world.get_component(current_job_eid_val, "Job") {
                    let job_state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                    if !matches!(
                        job_state,
                        "complete" | "failed" | "cancelled" | "interrupted" | "blocked"
                    ) {
                        let mut job_clone = job.clone();
                        job_clone["assigned_to"] = serde_json::Value::Null;
                        job_clone["state"] = serde_json::json!("pending");
                        world
                            .set_component(current_job_eid_val, "Job", job_clone)
                            .unwrap();
                        preempted_jobs.insert(*agent_id, current_job_eid_val);
                    }
                }
                if let Some(job) = world.get_component(new_job_eid, "Job") {
                    let mut job_clone = job.clone();
                    job_clone["assigned_to"] = serde_json::json!(*agent_id);
                    world.set_component(new_job_eid, "Job", job_clone).unwrap();
                }
                if let Some(agent_entry) =
                    world.components.get("Agent").and_then(|m| m.get(agent_id))
                {
                    let mut agent_obj = agent_entry.clone();
                    agent_obj["current_job"] = serde_json::json!(new_job_eid);
                    agent_obj["state"] = serde_json::json!("working");
                    world.set_component(*agent_id, "Agent", agent_obj).unwrap();
                }
                jobs_to_remove.push(new_job_eid);
                assigned_jobs.insert(new_job_eid);
                job_board.jobs.retain(|eid| *eid != new_job_eid);
                preempted_this_tick = true;
            }
        }

        if became_idle_this_tick || preempted_this_tick {
            let agent = match world.components.get("Agent").and_then(|m| m.get(agent_id)) {
                Some(agent) => agent,
                None => continue,
            };
            agent_state = agent
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("idle")
                .to_string();
            has_current_job = agent.get("current_job").and_then(|v| v.as_u64()).is_some();
            if preempted_this_tick {
                continue;
            }
        }

        if agent_state != "idle" {
            continue;
        }

        let mut assigned_job = None;
        let mut new_queue = agent_queue.clone();
        let agent = world.get_component(*agent_id, "Agent").unwrap();
        let specializations = agent
            .get("specializations")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

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
                    let job_category = job.get("category").and_then(|v| v.as_str()).unwrap_or("");
                    let job_state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                    if job_state == "pending"
                        && (!job_category.is_empty() && specializations.contains(&job_category))
                        && job_state != "blocked"
                        && job_state != "failed"
                        && job_state != "complete"
                        && job_state != "cancelled"
                        && job_state != "interrupted"
                    {
                        assigned_job = Some(next_job_eid);
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
            for &job_eid in &job_board.jobs {
                if assigned_jobs.contains(&job_eid) {
                    continue;
                }
                if let Some(job) = world.get_component(job_eid, "Job") {
                    let job_category = job.get("category").and_then(|v| v.as_str()).unwrap_or("");
                    let job_state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                    if job_state == "pending"
                        && (!job_category.is_empty() && specializations.contains(&job_category))
                        && job_state != "blocked"
                        && job_state != "failed"
                        && job_state != "complete"
                        && job_state != "cancelled"
                        && job_state != "interrupted"
                    {
                        assigned_job = Some(job_eid);
                        break;
                    }
                }
            }
        }

        if let Some(job_eid) = assigned_job {
            if let Some(job) = world.get_component(job_eid, "Job") {
                let mut job_clone = job.clone();
                job_clone["assigned_to"] = serde_json::json!(*agent_id);
                world
                    .set_component(job_eid, "Job", job_clone.clone())
                    .unwrap();

                crate::systems::job::system::events::emit_job_event(
                    world,
                    "job_assigned",
                    &job_clone,
                    None,
                );
            }
            if let Some(agent_entry) = world.components.get("Agent").and_then(|m| m.get(agent_id)) {
                let mut agent_obj = agent_entry.clone();
                agent_obj["current_job"] = serde_json::json!(job_eid);
                agent_obj["state"] = serde_json::json!("working");
                agent_obj["job_queue"] = serde_json::json!(new_queue);
                world.set_component(*agent_id, "Agent", agent_obj).unwrap();
            }
            jobs_to_remove.push(job_eid);
            assigned_jobs.insert(job_eid);
            job_board.jobs.retain(|eid| *eid != job_eid);
        }
    }

    // === Second pass: fallback assignment for any idle agent and unassigned job ===
    for agent_id in &agent_ids {
        let agent = match world.components.get("Agent").and_then(|m| m.get(agent_id)) {
            Some(agent) => agent,
            None => continue,
        };
        let agent_state = agent
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("idle");
        if agent_state != "idle" {
            continue;
        }
        let has_job = agent.get("current_job").and_then(|v| v.as_u64()).is_some();
        if has_job {
            continue;
        }
        let mut assigned_job = None;
        for &job_eid in &job_board.jobs {
            if assigned_jobs.contains(&job_eid) {
                continue;
            }
            if preempted_jobs.get(agent_id).copied() == Some(job_eid) {
                continue;
            }
            if let Some(job) = world.get_component(job_eid, "Job") {
                let job_state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                if job_state == "pending"
                    && job_state != "blocked"
                    && job_state != "failed"
                    && job_state != "complete"
                    && job_state != "cancelled"
                    && job_state != "interrupted"
                {
                    assigned_job = Some(job_eid);
                    break;
                }
            }
        }
        if let Some(job_eid) = assigned_job {
            if let Some(job) = world.get_component(job_eid, "Job") {
                let mut job_clone = job.clone();
                job_clone["assigned_to"] = serde_json::json!(*agent_id);
                world
                    .set_component(job_eid, "Job", job_clone.clone())
                    .unwrap();

                crate::systems::job::system::events::emit_job_event(
                    world,
                    "job_assigned",
                    &job_clone,
                    None,
                );
            }
            if let Some(agent_entry) = world.components.get("Agent").and_then(|m| m.get(agent_id)) {
                let mut agent_obj = agent_entry.clone();
                agent_obj["current_job"] = serde_json::json!(job_eid);
                agent_obj["state"] = serde_json::json!("working");
                world.set_component(*agent_id, "Agent", agent_obj).unwrap();
            }
            jobs_to_remove.push(job_eid);
            assigned_jobs.insert(job_eid);
            job_board.jobs.retain(|eid| *eid != job_eid);
        }
    }

    for agent_id in &agent_ids {
        if let Some(agent_entry) = world.components.get("Agent").and_then(|m| m.get(agent_id)) {
            let mut agent_obj = agent_entry.clone();
            let has_job = agent_obj
                .get("current_job")
                .and_then(|v| v.as_u64())
                .is_some();
            if !has_job {
                agent_obj["current_job"] = serde_json::Value::Null;
                world.set_component(*agent_id, "Agent", agent_obj).unwrap();
            }
        }
    }
}

/// Registers AI event subscriptions for resource shortages.
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
