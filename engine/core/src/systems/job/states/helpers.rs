//! Shared helpers for job state handlers: cancellation, pausing, and pathfinding failure.

use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// Returns `true` if the job is currently paused or interrupted.
pub fn try_handle_paused_or_interrupted(job: &JsonValue) -> bool {
    let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
    state == "paused" || state == "interrupted"
}

/// Handles job cancellation cleanup if required.
/// Returns `true` if cancellation was handled and the job should not be processed further.
pub fn try_handle_cancellation(world: &mut World, job: &mut JsonValue) -> bool {
    if job
        .get("cancelled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        && !job
            .get("cancelled_cleanup_done")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    {
        handle_job_cancellation_cleanup(world, job);
        job["state"] = serde_json::json!("cancelled");
        job["cancelled_cleanup_done"] = serde_json::json!(true);
        return true;
    }
    false
}

/// Drops all carried resources at the agent's position and removes them from the agent.
pub fn handle_job_cancellation_cleanup(world: &mut World, job: &JsonValue) {
    let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    if assigned_to != 0 {
        if let Some(mut agent) = world.get_component(assigned_to, "Agent").cloned() {
            if let Some(carried) = agent.get("carried_resources").cloned() {
                let agent_pos = world.get_component(assigned_to, "Position").cloned();
                if let Some(carried_arr) = carried.as_array() {
                    for res in carried_arr {
                        let kind = res
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let amount = res.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
                        let item_id = world.spawn_entity();
                        world
                            .set_component(
                                item_id,
                                "Item",
                                serde_json::json!({
                                    "id": item_id.to_string(),
                                    "name": format!("{} (loose)", kind),
                                    "kind": kind,
                                    "amount": amount,
                                    "loose": true,
                                    "slot": "loose"
                                }),
                            )
                            .unwrap();
                        if let Some(pos) = &agent_pos {
                            world
                                .set_component(item_id, "Position", pos.clone())
                                .unwrap();
                        }
                    }
                }
                agent.as_object_mut().unwrap().remove("carried_resources");
                let _ = world.set_component(assigned_to, "Agent", agent);
            }
        }
    }
}

/// Handles the case where no path can be found to the job site.
/// Sets job state to "blocked", unassigns the agent, and emits a "job_blocked" event.
pub fn handle_pathfinding_failure(world: &mut World, _eid: u32, mut job: JsonValue) -> JsonValue {
    job["state"] = serde_json::json!("blocked");
    if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64()) {
        let agent_id = agent_id as u32;
        if let Some(mut agent) = world.get_component(agent_id, "Agent").cloned() {
            agent.as_object_mut().unwrap().remove("current_job");
            agent["state"] = serde_json::json!("idle");
            let _ = world.set_component(agent_id, "Agent", agent);
        }
        job.as_object_mut().unwrap().remove("assigned_to");
    }
    crate::systems::job::system::JobSystem::emit_job_event(world, "job_blocked", &job, None);
    job
}
