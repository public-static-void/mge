//! State handlers for resource-related job states.
//!
//! These handlers orchestrate the agent's resource fetching and delivery process,
//! delegating low-level logic to resource and movement operation modules.

use super::helpers::*;
use crate::ecs::world::World;
use crate::systems::job::movement_ops;
use crate::systems::job::resource_ops;
use crate::systems::job::state_utils;
use serde_json::{Value as JsonValue, json};

/// Handles the "fetching_resources" state: agent picks up as much as possible from stockpile.
///
/// Ensures the agent moves to the stockpile, picks up resources if possible,
/// and transitions to "delivering_resources" or "waiting_for_resources".
pub fn handle_fetching_resources_state(
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
    let reserved_stockpile = job
        .get("reserved_stockpile")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    if assigned_to != 0 && reserved_stockpile.is_some() {
        let agent_pos = world.get_component(assigned_to, "Position");
        let stockpile_pos = world.get_component(reserved_stockpile.unwrap(), "Position");
        if let (Some(agent_pos), Some(stockpile_pos)) = (agent_pos, stockpile_pos) {
            let agent_cell = crate::map::CellKey::from_position(agent_pos);
            let stockpile_cell = crate::map::CellKey::from_position(stockpile_pos);

            if let (Some(agent_cell), Some(stockpile_cell)) = (agent_cell, stockpile_cell) {
                if agent_cell != stockpile_cell {
                    if movement_ops::is_move_path_empty(world, assigned_to) {
                        movement_ops::assign_move_path(
                            world,
                            assigned_to,
                            &agent_cell,
                            &stockpile_cell,
                        );
                    }
                    job["state"] = json!("fetching_resources");
                    return job;
                } else {
                    // At stockpile: try to pick up as much as possible
                    let mut stockpile = world
                        .get_component(reserved_stockpile.unwrap(), "Stockpile")
                        .cloned()
                        .unwrap();
                    let pickup = resource_ops::calculate_pickup(
                        world,
                        assigned_to,
                        &requirements,
                        &job,
                        stockpile
                            .get_mut("resources")
                            .unwrap()
                            .as_object_mut()
                            .unwrap(),
                    );

                    if pickup.is_empty() {
                        // Can't pick up anything (encumbered or nothing available)
                        job["state"] = json!("waiting_for_resources");
                        return job;
                    }

                    resource_ops::apply_pickup(
                        world,
                        assigned_to,
                        &pickup,
                        reserved_stockpile.unwrap(),
                        stockpile.get("resources").unwrap().as_object().unwrap(),
                    );

                    // Set move_path to job site after pickup
                    if let Some(target_pos) = job.get("target_position") {
                        if let Some(agent_pos) = world.get_component(assigned_to, "Position") {
                            let agent_cell = crate::map::CellKey::from_position(agent_pos);
                            let target_cell = crate::map::CellKey::from_position(target_pos);
                            if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell)
                            {
                                if agent_cell != target_cell {
                                    movement_ops::assign_move_path(
                                        world,
                                        assigned_to,
                                        &agent_cell,
                                        &target_cell,
                                    );
                                }
                            }
                        }
                    }

                    job["state"] = json!("delivering_resources");
                    return job;
                }
            }
        }
    }
    job
}

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
                // At job site: deliver resources
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

                // Remove resources from agent
                agent.as_object_mut().unwrap().remove("carried_resources");
                let _ = world.set_component(assigned_to, "Agent", agent);

                // Check if all requirements are met
                if state_utils::are_requirements_met(
                    &requirements,
                    job["delivered_resources"].as_array().unwrap(),
                ) {
                    job["state"] = json!("in_progress");
                } else {
                    // Need more deliveries: go back to fetching_resources
                    job["state"] = json!("fetching_resources");
                }
                return job;
            }
        }
    }
    job
}
