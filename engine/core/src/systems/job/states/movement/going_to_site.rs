//! Handler for the "going_to_site" job state.

use crate::ecs::world::World;
use crate::systems::job::movement_ops;
use crate::systems::job::states::helpers::*;
use serde_json::Value as JsonValue;

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
                world.set_component(eid, "Job", job.clone()).unwrap();
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
