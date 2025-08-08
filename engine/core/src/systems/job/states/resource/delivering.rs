//! Handler for the "delivering_resources" job state.

use crate::ecs::world::World;
use crate::systems::job::movement_ops;
use crate::systems::job::resource_ops;
use crate::systems::job::states::helpers::*;
use crate::systems::job::states::transitions;
use serde_json::{Value as JsonValue, json};

/// Handles the "delivering_resources" state: agent delivers carried resources to job site.
///
/// Ensures the agent moves to the job site, delivers resources if at the site,
/// and transitions to "in_progress" or "fetching_resources" as appropriate.
pub fn handle_delivering_resources_state(
    world: &mut World,
    _eid: u32,
    mut job: JsonValue,
) -> JsonValue {
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
    let agent_pos = world.get_component(assigned_to, "Position");
    let target_pos = job.get("target_position");
    if let (Some(agent_pos), Some(target_pos)) = (agent_pos, target_pos) {
        let agent_cell = crate::map::CellKey::from_position(agent_pos);
        let target_cell = crate::map::CellKey::from_position(target_pos);
        if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell) {
            if agent_cell != target_cell {
                if movement_ops::is_move_path_empty(world, assigned_to) {
                    movement_ops::assign_move_path(world, assigned_to, &agent_cell, &target_cell);
                }
                job["state"] = json!("delivering_resources");
                return job;
            } else {
                // At job site: deliver resources and update delivered_resources
                let mut agent = world.get_component(assigned_to, "Agent").cloned().unwrap();
                let carried = agent.get("carried_resources").cloned().unwrap_or(json!([]));
                let delivered = job
                    .get("delivered_resources")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_else(Vec::new);

                let new_delivered = resource_ops::accumulate_delivery(
                    &requirements,
                    &delivered,
                    carried.as_array().unwrap_or(&vec![]),
                );
                job["delivered_resources"] = JsonValue::Array(new_delivered);

                // Clear agent's carried resources after delivery
                agent.as_object_mut().unwrap().remove("carried_resources");
                let _ = world.set_component(assigned_to, "Agent", agent);

                // If all requirements are delivered, transition to in_progress for job completion work
                if transitions::are_requirements_met(
                    &requirements,
                    job["delivered_resources"].as_array().unwrap_or(&vec![]),
                ) {
                    if job.get("assigned_to").and_then(|v| v.as_u64()).is_some() {
                        job["state"] = json!("in_progress");
                    }
                } else {
                    // More deliveries needed: go back to fetching_resources
                    job["state"] = json!("fetching_resources");
                }
                return job;
            }
        }
    }
    job
}
