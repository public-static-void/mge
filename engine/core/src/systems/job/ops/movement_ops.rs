//! Pathfinding and movement utilities for the job system.
//!
//! This module provides reusable functions for assigning move paths to agents,
//! checking agent location, and managing movement-related logic for jobs.

use crate::ecs::world::World;
use serde_json::{Value as JsonValue, json};

/// Assigns a move path to the agent from `from_cell` to `to_cell` using the map's pathfinding.
/// If a valid path exists, updates the agent's `move_path` component.
pub fn assign_move_path(
    world: &mut World,
    agent_id: u32,
    from_cell: &crate::map::CellKey,
    to_cell: &crate::map::CellKey,
) {
    if let Some(map) = &world.map
        && let Some(pathfinding) = map.find_path(from_cell, to_cell) {
            if pathfinding.path.len() <= 1 {
                // Already at destination or path empty; clear move_path if any
                let mut agent = world.get_component(agent_id, "Agent").cloned().unwrap();
                agent.as_object_mut().unwrap().remove("move_path");
                let _ = world.set_component(agent_id, "Agent", agent);
                return;
            }
            let move_path: Vec<JsonValue> = pathfinding
                .path
                .iter()
                .skip(1)
                .map(|cell| match cell {
                    crate::map::CellKey::Square { x, y, z } => {
                        json!({ "Square": { "x": x, "y": y, "z": z } })
                    }
                    crate::map::CellKey::Hex { q, r, z } => {
                        json!({ "Hex": { "q": q, "r": r, "z": z } })
                    }
                    crate::map::CellKey::Region { id } => {
                        json!({ "Region": { "id": id } })
                    }
                })
                .collect();
            let mut agent = world.get_component(agent_id, "Agent").cloned().unwrap();
            agent["move_path"] = json!(move_path);
            let _ = world.set_component(agent_id, "Agent", agent);
        }
}

/// Returns `true` if the agent is currently at the given cell.
pub fn is_agent_at_cell(world: &World, agent_id: u32, cell: &crate::map::CellKey) -> bool {
    if let Some(agent_pos) = world.get_component(agent_id, "Position")
        && let Some(agent_cell) = crate::map::CellKey::from_position(agent_pos) {
            return &agent_cell == cell;
        }
    false
}

/// Returns `true` if the agent's move_path is empty or not set.
pub fn is_move_path_empty(world: &World, agent_id: u32) -> bool {
    if let Some(agent) = world.get_component(agent_id, "Agent") {
        match agent.get("move_path") {
            None => true,
            Some(v) => v.as_array().map(|a| a.is_empty()).unwrap_or(true),
        }
    } else {
        true
    }
}
