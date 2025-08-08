//! Core job processing logic for the job system.

use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// Handles job progress and invokes custom or default logic as appropriate.
pub fn process_job_progress(
    world: &mut World,
    eid: u32,
    job_type: String,
    job: serde_json::Value,
) -> serde_json::Value {
    let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");

    if !matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
        let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let handler_opt = {
            let registry = world.job_handler_registry.lock().unwrap();
            registry.get(&job_type).cloned()
        };
        if let Some(handler) = handler_opt {
            let prev_progress = job.get("progress").and_then(|v| v.as_f64());
            let result = handler(world, assigned_to, job_id, &job);
            let new_progress = result.get("progress").and_then(|v| v.as_f64());
            if new_progress != prev_progress {
                crate::systems::job::system::events::emit_job_event(
                    world,
                    "job_progressed",
                    &result,
                    None,
                );
            }
            world.set_component(eid, "Job", result.clone()).unwrap();

            // If a handler is registered and handled the state, do NOT apply default progress again!
            // The handler is fully responsible for progress of this job_type and state.
            return result;
        }
    }

    match state {
        "pending" => crate::systems::job::states::handle_pending_state(world, eid, job),
        "going_to_site" => crate::systems::job::states::handle_going_to_site_state(world, eid, job),
        "fetching_resources" => {
            crate::systems::job::states::handle_fetching_resources_state(world, eid, job)
        }
        "delivering_resources" => {
            crate::systems::job::states::handle_delivering_resources_state(world, eid, job)
        }
        "at_site" => crate::systems::job::states::handle_at_site_state(world, eid, job),
        "in_progress" => default_job_progress(world, eid, job_type, job),
        _ => job,
    }
}

/// The default job progress logic for jobs in "in_progress" state.
///
/// This function ensures jobs make progress whether or not an agent is assigned,
/// so that effect-only jobs and agent-driven jobs both work properly.
fn default_job_progress(
    world: &mut World,
    _eid: u32,
    job_type: String,
    mut job: JsonValue,
) -> JsonValue {
    let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    let mut at_site = true;
    if assigned_to != 0
        && let Some(target_pos) = job.get("target_position")
        && let Some(agent_pos) = world.get_component(assigned_to, "Position")
    {
        let agent_cell = crate::map::CellKey::from_position(agent_pos);
        let target_cell = crate::map::CellKey::from_position(target_pos);
        if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell)
            && agent_cell != target_cell
        {
            at_site = false;
        }
    }
    if !at_site {
        return job;
    }

    // Always increment progress, even for jobs without an agent.
    let mut progress_increment = 1.0;
    if assigned_to != 0
        && let Some(agent) = world.get_component(assigned_to, "Agent")
    {
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
    let prev_progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let progress = prev_progress + progress_increment;
    job["progress"] = serde_json::json!(progress);
    if progress != prev_progress {
        crate::systems::job::system::events::emit_job_event(world, "job_progressed", &job, None);
    }
    let required_progress = job
        .get("required_progress")
        .and_then(|v| v.as_f64())
        .unwrap_or(3.0);
    if progress >= required_progress {
        job["progress"] = serde_json::json!(progress.max(required_progress));
        job["state"] = serde_json::json!("complete");
    }
    job
}

/// Processes a single job, updating its state and handling dependencies, cancellation, and children.
pub fn process_job(
    world: &mut World,
    _lua: Option<&mlua::Lua>,
    eid: u32,
    mut job: serde_json::Value,
) -> serde_json::Value {
    let job_type = job
        .get("job_type")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let cancelled_cleanup_done = job
        .get("cancelled_cleanup_done")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // If job is in a terminal state, never allow it to transition out.
    {
        let state = job
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if matches!(
            state.as_str(),
            "cancelled" | "complete" | "failed" | "interrupted" | "blocked"
        ) {
            if state == "cancelled" && !cancelled_cleanup_done {
                if job.get("children").is_some() {
                    let children_val = job.get_mut("children").unwrap();
                    let (new_children, _all_children_complete) =
                        crate::systems::job::children::process_job_children(
                            world,
                            _lua,
                            eid,
                            children_val,
                            true,
                        );
                    job["children"] = new_children;
                }
                crate::systems::job::states::handle_job_cancellation_cleanup(world, &job);
                crate::systems::job::system::orchestrator::cleanup_agent_on_job_state(world, &job);
                world.set_component(eid, "Job", job.clone()).unwrap();
                crate::systems::job::system::events::emit_job_event(
                    world,
                    "job_cancelled",
                    &job,
                    None,
                );
            }
            if state == "complete"
                && let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64())
                && let Some(mut agent) = world.get_component(agent_id as u32, "Agent").cloned()
            {
                agent["current_job"] = serde_json::Value::Null;
                agent["state"] = serde_json::json!("idle");
                world
                    .set_component(agent_id as u32, "Agent", agent)
                    .unwrap();
            }
            return job;
        }
    }

    // If agent is missing, mark job as interrupted and clean up assignment.
    if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64()) {
        let agent_id = agent_id as u32;
        if world.get_component(agent_id, "Agent").is_none() {
            job["state"] = serde_json::json!("interrupted");
            let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(eid as u64) as u32;
            if let Some(obj) = job.as_object_mut() {
                crate::systems::job::system::effects::process_job_effects(
                    world, job_id, &job_type, obj, true,
                );
            }
            job["assigned_to"] = serde_json::Value::Null;
            world.set_component(eid, "Job", job.clone()).unwrap();
            return job;
        }
    }

    // Dependency failure handling.
    if let Some(dep_fail_state) =
        crate::systems::job::dependencies::dependency_failure_state(world, &job)
    {
        job["state"] = serde_json::json!(dep_fail_state);

        if dep_fail_state == "failed"
            && let Some(to_spawn) = job
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
            job["children"] = serde_json::Value::Array(children);
        }
        world.set_component(eid, "Job", job.clone()).unwrap();
        return job;
    }

    // If dependencies are not satisfied, set state to pending.
    if !crate::systems::job::dependencies::dependencies_satisfied(world, &job) {
        if job.get("state").and_then(|v| v.as_str()) != Some("pending") {
            job["state"] = serde_json::json!("pending");
        }
        world.set_component(eid, "Job", job.clone()).unwrap();
        return job;
    }

    // Promote job state from "pending" to "in_progress" if:
    // - the job is unassigned, OR
    // - the job does NOT have a target position
    //
    // For assigned jobs with a target position, defer state advancement
    // to allow the pending state handler to set "going_to_site" first.
    if job.get("state").and_then(|v| v.as_str()) == Some("pending") {
        let has_target_position = job.get("target_position").is_some();
        let assigned_to_some = job.get("assigned_to").and_then(|v| v.as_u64()).is_some();

        if assigned_to_some && has_target_position {
            // Defer state advancement, let handle_pending_state decide
        } else {
            // For unassigned or no-target jobs, move to in_progress immediately
            job["state"] = serde_json::json!("in_progress");
        }
    }

    let state_is_cancelled = {
        let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
        state == "cancelled"
    };

    // Update children array if present.
    if job.get("children").is_some() {
        let children_val = job.get_mut("children").unwrap();
        let (new_children, all_children_complete) =
            crate::systems::job::children::process_job_children(
                world,
                _lua,
                eid,
                children_val,
                state_is_cancelled,
            );
        job["children"] = new_children;
        if all_children_complete {
            job["state"] = serde_json::json!("complete");
        }
        world.set_component(eid, "Job", job.clone()).unwrap();
    } else {
        world.set_component(eid, "Job", job.clone()).unwrap();
    }

    // Fail job if should_fail flag is set.
    if job
        .get("should_fail")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        job["state"] = serde_json::json!("failed");
        world.set_component(eid, "Job", job.clone()).unwrap();
        return job;
    }

    // Allow effect-only jobs (with no assigned_to) to stay in "in_progress" state
    // Revert to pending only if no assigned_to AND no effects defined
    if job.get("state").and_then(|v| v.as_str()) == Some("in_progress") {
        let has_assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).is_some();
        let has_effects = world
            .job_types
            .get_data(job.get("job_type").and_then(|v| v.as_str()).unwrap_or(""))
            .map(|jt| !jt.effects.is_empty())
            .unwrap_or(false);

        if !has_assigned_to && !has_effects {
            job["state"] = serde_json::json!("pending");
            world.set_component(eid, "Job", job.clone()).unwrap();
            return job;
        }
    }

    // Track previous state to detect transition to "complete"
    let prev_state = job
        .get("state")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // State machine for job progress.
    let mut job = match job.get("state").and_then(|v| v.as_str()) {
        Some("pending") | None => {
            crate::systems::job::states::handle_pending_state(world, eid, job)
        }
        Some("going_to_site") => {
            crate::systems::job::states::handle_going_to_site_state(world, eid, job)
        }
        Some("fetching_resources") => {
            crate::systems::job::states::handle_fetching_resources_state(world, eid, job)
        }
        Some("delivering_resources") => {
            crate::systems::job::states::handle_delivering_resources_state(world, eid, job)
        }
        Some("at_site") => crate::systems::job::states::handle_at_site_state(world, eid, job),
        _ => job,
    };
    world.set_component(eid, "Job", job.clone()).unwrap();

    // If job just transitioned to "complete", cleanup agent assignment and unassign job
    let current_state = job.get("state").and_then(|v| v.as_str());
    if current_state == Some("complete") && prev_state.as_deref() != Some("complete") {
        if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64())
            && let Some(mut agent) = world.get_component(agent_id as u32, "Agent").cloned()
        {
            agent["current_job"] = serde_json::Value::Null;
            agent["state"] = serde_json::json!("idle");
            world
                .set_component(agent_id as u32, "Agent", agent)
                .unwrap();
        }
        job["assigned_to"] = serde_json::Value::Null;
        world.set_component(eid, "Job", job.clone()).unwrap();
    }

    let job = process_job_progress(world, eid, job_type, job);
    world.set_component(eid, "Job", job.clone()).unwrap();

    job
}
