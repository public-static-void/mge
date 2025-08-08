//! Handler for the "pending" job state.

use crate::ecs::world::World;
use crate::systems::job::core::requirements;
use crate::systems::job::movement_ops;
use crate::systems::job::states::helpers::*;

/// Handles the "pending" state of a job, including transitions to fetching_resources or going_to_site.
///
/// This function checks for cancellation, pausing, and then determines if the job should
/// move to fetching resources, going to site, or directly to in_progress.
///
/// Adjusted to require movement phase if a target_position is defined, regardless of resource requirements.
pub fn handle_pending_state(
    world: &mut World,
    eid: u32,
    mut job: serde_json::Value,
) -> serde_json::Value {
    // Skip terminal or paused states early
    let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
    if matches!(state, "blocked" | "cancelled" | "failed" | "complete") {
        return job;
    }
    if try_handle_paused_or_interrupted(&job) {
        return job;
    }
    if try_handle_cancellation(world, &mut job) {
        return job;
    }

    let assigned_to_u64 = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0);
    let assigned_to: u32 = assigned_to_u64.try_into().unwrap_or(0);

    if assigned_to == 0 {
        return job;
    }

    let requirements = job
        .get("resource_requirements")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let target_position = job.get("target_position").cloned();
    let target_position_is_some = match &target_position {
        Some(val) => !val.is_null(),
        None => false,
    };

    if !requirements.is_empty()
        && job.get("reserved_resources").is_some()
        && job
            .get("reserved_stockpile")
            .and_then(|v| v.as_u64())
            .is_some()
    {
        job["state"] = serde_json::json!("fetching_resources");
        world.set_component(eid, "Job", job.clone()).unwrap();
        return job;
    }

    if target_position_is_some {
        let agent_pos = world.get_component(assigned_to, "Position");
        if let (Some(agent_pos), Some(target_pos)) = (agent_pos, target_position.as_ref()) {
            let agent_cell = crate::map::CellKey::from_position(agent_pos);
            let target_cell = crate::map::CellKey::from_position(target_pos);
            if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell) {
                if agent_cell == target_cell {
                    if !requirements.is_empty()
                        && job.get("reserved_resources").is_some()
                        && job
                            .get("reserved_stockpile")
                            .and_then(|v| v.as_u64())
                            .is_some()
                    {
                        job["state"] = serde_json::json!("fetching_resources");
                    } else {
                        job["state"] = serde_json::json!("in_progress");
                    }
                    world.set_component(eid, "Job", job.clone()).unwrap();
                    return job;
                } else {
                    if movement_ops::is_move_path_empty(world, assigned_to) {
                        movement_ops::assign_move_path(
                            world,
                            assigned_to,
                            &agent_cell,
                            &target_cell,
                        );
                    }
                    job["state"] = serde_json::json!("going_to_site");
                    world.set_component(eid, "Job", job.clone()).unwrap();
                    return job;
                }
            }
        }
        return job;
    }

    if requirements::requirements_are_empty_or_zero(&requirements) {
        job["state"] = serde_json::json!("in_progress");
        world.set_component(eid, "Job", job.clone()).unwrap();
        return job;
    }

    job
}
