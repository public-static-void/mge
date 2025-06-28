//! Core job processing logic for the job system.

use crate::ecs::world::World;
use crate::systems::job::{children, dependencies, states};
use serde_json::Value as JsonValue;

/// Processes a single job, updating its state and handling dependencies, cancellation, and children.
pub fn process_job(
    world: &mut World,
    _lua: Option<&mlua::Lua>,
    eid: u32,
    mut job: JsonValue,
) -> JsonValue {
    let job_type = job
        .get("job_type")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let is_cancelled = job
        .get("cancelled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let cancelled_cleanup_done = job
        .get("cancelled_cleanup_done")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // If job is assigned to an agent, but agent does not exist, interrupt and rollback
    if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64()) {
        let agent_id = agent_id as u32;
        if world.get_component(agent_id, "Agent").is_none() {
            // Mark as interrupted, rollback all applied effects
            job["state"] = serde_json::json!("interrupted");
            let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(eid as u64) as u32;
            if let Some(obj) = job.as_object_mut() {
                crate::systems::job::system::effects::process_job_effects(
                    world, job_id, &job_type, obj, true,
                );
            }
            job.as_object_mut().unwrap().remove("assigned_to");
            world.set_component(eid, "Job", job.clone()).unwrap();
            return job;
        }
    }

    if let Some(dep_fail_state) = dependencies::dependency_failure_state(world, &job) {
        job["state"] = serde_json::json!(dep_fail_state);

        if dep_fail_state == "failed" {
            if let Some(to_spawn) = job
                .get("on_dependency_failed_spawn")
                .and_then(|v| v.as_array())
            {
                let mut children = job
                    .get("children")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                for child in to_spawn {
                    children.push(child.clone());
                }
                job["children"] = JsonValue::Array(children);
            }
        }
        return job;
    }

    if !dependencies::dependencies_satisfied(world, &job) {
        if job.get("state").and_then(|v| v.as_str()) != Some("pending") {
            job["state"] = serde_json::json!("pending");
        }
        return job;
    }

    if is_cancelled && !cancelled_cleanup_done {
        job["state"] = serde_json::json!("cancelled");
        crate::systems::job::states::handle_job_cancellation_cleanup(world, &job);
        job["cancelled_cleanup_done"] = serde_json::json!(true);
        world.set_component(eid, "Job", job.clone()).unwrap();
    }

    if let Some(children_val) = job.get_mut("children") {
        let (new_children, all_children_complete) =
            children::process_job_children(world, _lua, eid, children_val, is_cancelled);
        *children_val = new_children;
        if all_children_complete {
            job["state"] = serde_json::json!("complete");
        }
    }

    if job
        .get("should_fail")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        job["state"] = serde_json::json!("failed");
        return job;
    }

    let job = match job.get("state").and_then(|v| v.as_str()) {
        Some("pending") | None => states::handle_pending_state(world, eid, job),
        Some("going_to_site") => states::handle_going_to_site_state(world, eid, job),
        Some("fetching_resources") => states::handle_fetching_resources_state(world, eid, job),
        Some("delivering_resources") => states::handle_delivering_resources_state(world, eid, job),
        Some("at_site") => states::handle_at_site_state(world, eid, job),
        _ => job,
    };
    world.set_component(eid, "Job", job.clone()).unwrap();
    crate::systems::job::system::process::process_job_progress(world, eid, job_type, job)
}

/// Handles job progress and invokes custom or default logic as appropriate.
pub fn process_job_progress(
    world: &mut World,
    eid: u32,
    job_type: String,
    mut job: JsonValue,
) -> JsonValue {
    let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");

    // Custom handler always takes priority
    if !matches!(state, "failed" | "complete" | "cancelled") {
        let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let registry = world.job_handler_registry.lock().unwrap();
        if let Some(handler) = registry.get(&job_type) {
            let prev_progress = job.get("progress").and_then(|v| v.as_f64());
            let result = handler(world, assigned_to, job_id, &job);
            let new_progress = result.get("progress").and_then(|v| v.as_f64());
            drop(registry);
            if new_progress != prev_progress {
                crate::systems::job::system::events::emit_job_event(
                    world,
                    "job_progressed",
                    &result,
                    None,
                );
            }
            return result;
        }
    }

    match state {
        "pending" => states::handle_pending_state(world, eid, job),
        "going_to_site" => states::handle_going_to_site_state(world, eid, job),
        "fetching_resources" => states::handle_fetching_resources_state(world, eid, job),
        "delivering_resources" => states::handle_delivering_resources_state(world, eid, job),
        "at_site" => states::handle_at_site_state(world, eid, job),
        "in_progress" => {
            // Extract job_type_str before mut borrow
            let job_type_str = job
                .get("job_type")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();
            let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(eid as u64) as u32;
            if let Some(obj) = job.as_object_mut() {
                crate::systems::job::system::effects::process_job_effects(
                    world,
                    job_id,
                    &job_type_str,
                    obj,
                    false,
                );
            }

            let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

            // Only progress if agent is at the site (if applicable)
            let mut at_site = true;
            if assigned_to != 0 {
                if let Some(target_pos) = job.get("target_position") {
                    if let Some(agent_pos) = world.get_component(assigned_to, "Position") {
                        let agent_cell = crate::map::CellKey::from_position(agent_pos);
                        let target_cell = crate::map::CellKey::from_position(target_pos);
                        if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell) {
                            if agent_cell != target_cell {
                                at_site = false;
                            }
                        }
                    }
                }
            }
            if !at_site {
                return job;
            }

            // Progress the job
            let mut progress_increment = 1.0;
            if assigned_to != 0 {
                if let Some(agent) = world.get_component(assigned_to, "Agent") {
                    let skills = agent.get("skills").and_then(|v| v.as_object());
                    let skill = skills
                        .and_then(|map| map.get(&job_type))
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0);
                    let stamina = agent
                        .get("stamina")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(100.0);
                    progress_increment = 1.0 * skill * (stamina / 100.0);
                    if progress_increment < 0.1 {
                        progress_increment = 0.1;
                    }
                }
            }
            let prev_progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let progress = prev_progress + progress_increment;
            job["progress"] = serde_json::json!(progress);
            if progress != prev_progress {
                crate::systems::job::system::events::emit_job_event(
                    world,
                    "job_progressed",
                    &job,
                    None,
                );
            }
            if progress >= 3.0 {
                job["progress"] = serde_json::json!(progress.max(3.0));
                job["state"] = serde_json::json!("complete");
            }
            job
        }
        _ => job,
    }
}
