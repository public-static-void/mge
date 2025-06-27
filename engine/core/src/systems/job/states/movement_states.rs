//! State handlers for movement/location-related job states.
//!
//! These handlers orchestrate the agent's movement to the job site and related transitions,
//! delegating low-level logic to movement operation and state utility modules.

use super::helpers::*;
use crate::ecs::world::World;
use crate::systems::job::movement_ops;
use crate::systems::job::requirements;
use crate::systems::job::state_utils;
use serde_json::{Value as JsonValue, json};

/// Handles the "at_site" state: transitions the job to "in_progress".
///
/// This state is entered when the agent arrives at the job's target location.
pub fn handle_at_site_state(_world: &mut World, _eid: u32, mut job: JsonValue) -> JsonValue {
    job["state"] = json!("in_progress");
    job
}

/// Handles the "pending" state of a job, including transitions to fetching_resources or going_to_site.
///
/// This function checks for cancellation, pausing, and then determines if the job should
/// move to fetching resources, going to site, or directly to in_progress.
pub fn handle_pending_state(world: &mut World, eid: u32, mut job: JsonValue) -> JsonValue {
    if try_handle_paused_or_interrupted(&job) {
        return job;
    }
    if try_handle_cancellation(world, &mut job) {
        return job;
    }

    let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let requirements = job
        .get("resource_requirements")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Transition: pending -> fetching_resources (for jobs WITH resources)
    if assigned_to != 0
        && job.get("reserved_resources").is_some()
        && job
            .get("reserved_stockpile")
            .and_then(|v| v.as_i64())
            .is_some()
        && !requirements.is_empty()
    {
        job["state"] = json!("fetching_resources");
        world.set_component(eid, "Job", job.clone()).unwrap();
        return job;
    }

    // Transition: pending -> in_progress or going_to_site
    if assigned_to != 0
        && (requirements::requirements_are_empty_or_zero(&requirements)
            || (requirements::is_reserved_resources_empty(&job)
                && requirements::reserved_stockpile_is_none_or_not_int(&job)))
        && job.get("target_position").is_some()
    {
        let agent_pos = world.get_component(assigned_to, "Position");
        let target_pos = job.get("target_position");
        if let (Some(agent_pos), Some(target_pos)) = (agent_pos, target_pos) {
            let agent_cell = crate::map::CellKey::from_position(agent_pos);
            let target_cell = crate::map::CellKey::from_position(target_pos);
            if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell) {
                if state_utils::transition_if_at_site(&agent_cell, &target_cell, &mut job) {
                    return job;
                } else {
                    if let Some(map) = &world.map {
                        if map.find_path(&agent_cell, &target_cell).is_none() {
                            return handle_pathfinding_failure(world, eid, job);
                        }
                    }
                    if movement_ops::is_move_path_empty(world, assigned_to) {
                        movement_ops::assign_move_path(
                            world,
                            assigned_to,
                            &agent_cell,
                            &target_cell,
                        );
                    }
                    job["state"] = json!("going_to_site");
                    return job;
                }
            }
        }
    }

    // If no requirements and no movement, start the job after dependencies are complete
    if requirements::requirements_are_empty_or_zero(&requirements)
        && job.get("target_position").is_none()
    {
        job["state"] = json!("in_progress");
        return job;
    }

    job
}

/// Handles the "going_to_site" state: agent moves toward the job site, or transitions to "at_site" if arrived.
///
/// This function checks for cancellation, pausing, and pathfinding failure. If the agent
/// is not at the target, it ensures a valid move path exists. If the agent is at the site,
/// the job transitions to "at_site".
pub fn handle_going_to_site_state(world: &mut World, eid: u32, mut job: JsonValue) -> JsonValue {
    if try_handle_paused_or_interrupted(&job) {
        return job;
    }
    if try_handle_cancellation(world, &mut job) {
        return job;
    }

    let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let agent_pos = world.get_component(assigned_to, "Position");
    let target_pos = job.get("target_position");
    if let (Some(agent_pos), Some(target_pos)) = (agent_pos, target_pos) {
        let agent_cell = crate::map::CellKey::from_position(agent_pos);
        let target_cell = crate::map::CellKey::from_position(target_pos);
        if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell) {
            if agent_cell == target_cell {
                job["state"] = serde_json::json!("at_site");
                return job;
            } else if movement_ops::is_move_path_empty(world, assigned_to) {
                if let Some(map) = &world.map {
                    let path_result = map.find_path(&agent_cell, &target_cell);
                    if path_result.is_none() {
                        return handle_pathfinding_failure(world, eid, job);
                    }
                    movement_ops::assign_move_path(world, assigned_to, &agent_cell, &target_cell);
                }
            }
        }
    }
    job
}
