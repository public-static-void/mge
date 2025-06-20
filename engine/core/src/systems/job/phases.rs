use crate::ecs::world::World;
use crate::systems::job::requirements;
use serde_json::Value as JsonValue;

/// Handles the "pending" phase, including transitions to fetching_resources or going_to_site.
/// Returns the updated job value.
pub fn handle_pending_phase(world: &mut World, eid: u32, mut job: JsonValue) -> JsonValue {
    let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let requirements = job
        .get("resource_requirements")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Phase transition: pending -> fetching_resources (for jobs WITH resources)
    if assigned_to != 0
        && job.get("reserved_resources").is_some()
        && job
            .get("reserved_stockpile")
            .and_then(|v| v.as_i64())
            .is_some()
        && !requirements.is_empty()
    {
        job["phase"] = serde_json::json!("fetching_resources");
        world.set_component(eid, "Job", job.clone()).unwrap();
        return job;
    }

    // Phase transition: pending -> going_to_site (for jobs WITHOUT resources)
    if assigned_to != 0
        && (requirements::requirements_are_empty_or_zero(&requirements)
            || (requirements::is_reserved_resources_empty(&job)
                && requirements::reserved_stockpile_is_none_or_not_int(&job)))
    {
        let agent_pos = world.get_component(assigned_to, "Position");
        let target_pos = job.get("target_position");
        if let (Some(agent_pos), Some(target_pos)) = (agent_pos, target_pos) {
            let agent_cell = crate::map::CellKey::from_position(agent_pos);
            let target_cell = crate::map::CellKey::from_position(target_pos);
            if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell) {
                if agent_cell != target_cell {
                    let mut agent = world.get_component(assigned_to, "Agent").cloned().unwrap();
                    let move_path_empty = match agent.get("move_path") {
                        None => true,
                        Some(v) => v.as_array().map(|a| a.is_empty()).unwrap_or(true),
                    };
                    if move_path_empty {
                        if let Some(map) = &world.map {
                            if let Some(pathfinding) = map.find_path(&agent_cell, &target_cell) {
                                let move_path: Vec<JsonValue> = pathfinding
                                    .path
                                    .into_iter()
                                    .skip(1)
                                    .map(|cell| match cell {
                                        crate::map::CellKey::Square { x, y, z } => {
                                            serde_json::json!({ "Square": { "x": x, "y": y, "z": z } })
                                        }
                                        crate::map::CellKey::Hex { q, r, z } => {
                                            serde_json::json!({ "Hex": { "q": q, "r": r, "z": z } })
                                        }
                                        crate::map::CellKey::Region { ref id } => {
                                            serde_json::json!({ "Region": { "id": id } })
                                        }
                                    })
                                    .collect();
                                agent["move_path"] = serde_json::json!(move_path);
                                let _ = world.set_component(assigned_to, "Agent", agent);
                            } else {
                                return handle_pathfinding_failure(world, eid, job);
                            }
                        }
                    }
                    job["phase"] = serde_json::json!("going_to_site");
                    return job;
                } else {
                    job["phase"] = serde_json::json!("at_site");
                    return job;
                }
            }
        }
    }

    // If no requirements and no movement, start the job after dependencies are complete
    if requirements::requirements_are_empty_or_zero(&requirements)
        && job.get("target_position").is_none()
    {
        job["phase"] = serde_json::json!("in_progress");
        return job;
    }

    job
}

/// Handles the "going_to_site" phase.
/// Returns the updated job value.
pub fn handle_going_to_site_phase(world: &mut World, _eid: u32, mut job: JsonValue) -> JsonValue {
    let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let agent_pos = world.get_component(assigned_to, "Position");
    let target_pos = job.get("target_position");
    if let (Some(agent_pos), Some(target_pos)) = (agent_pos, target_pos) {
        let agent_cell = crate::map::CellKey::from_position(agent_pos);
        let target_cell = crate::map::CellKey::from_position(target_pos);
        if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell) {
            if agent_cell == target_cell {
                job["phase"] = serde_json::json!("at_site");
                return job;
            }
        }
    }
    job
}

/// Handles the "fetching_resources" phase.
/// Returns the updated job value.
pub fn handle_fetching_resources_phase(
    world: &mut World,
    _eid: u32,
    mut job: JsonValue,
) -> JsonValue {
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
                    let mut agent = world.get_component(assigned_to, "Agent").cloned().unwrap();
                    let move_path_empty = match agent.get("move_path") {
                        None => true,
                        Some(v) => v.as_array().map(|a| a.is_empty()).unwrap_or(true),
                    };
                    if move_path_empty {
                        if let Some(map) = &world.map {
                            if let Some(pathfinding) = map.find_path(&agent_cell, &stockpile_cell) {
                                let mut move_path: Vec<JsonValue> = pathfinding
                                    .path
                                    .into_iter()
                                    .skip(1)
                                    .map(|cell| match cell {
                                        crate::map::CellKey::Square { x, y, z } => {
                                            serde_json::json!({ "Square": { "x": x, "y": y, "z": z } })
                                        }
                                        crate::map::CellKey::Hex { q, r, z } => {
                                            serde_json::json!({ "Hex": { "q": q, "r": r, "z": z } })
                                        }
                                        crate::map::CellKey::Region { ref id } => {
                                            serde_json::json!({ "Region": { "id": id } })
                                        }
                                    })
                                    .collect();
                                if move_path.is_empty() && agent_cell != stockpile_cell {
                                    move_path.push(match stockpile_cell {
                                        crate::map::CellKey::Square { x, y, z } => {
                                            serde_json::json!({ "Square": { "x": x, "y": y, "z": z } })
                                        }
                                        crate::map::CellKey::Hex { q, r, z } => {
                                            serde_json::json!({ "Hex": { "q": q, "r": r, "z": z } })
                                        }
                                        crate::map::CellKey::Region { ref id } => {
                                            serde_json::json!({ "Region": { "id": id } })
                                        }
                                    });
                                }
                                agent["move_path"] = serde_json::json!(move_path);
                                let _ = world.set_component(assigned_to, "Agent", agent);
                            }
                        }
                    }
                    job["phase"] = serde_json::json!("fetching_resources");
                    return job;
                } else {
                    let mut agent = world.get_component(assigned_to, "Agent").cloned().unwrap();
                    agent["carried_resources"] = serde_json::json!(requirements);
                    let _ = world.set_component(assigned_to, "Agent", agent);
                    job["phase"] = serde_json::json!("delivering_resources");
                    return job;
                }
            }
        }
    }
    job
}

/// Handles the "delivering_resources" phase.
/// Returns the updated job value.
pub fn handle_delivering_resources_phase(
    world: &mut World,
    _eid: u32,
    mut job: JsonValue,
) -> JsonValue {
    let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let _requirements = job
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
                let mut agent = world.get_component(assigned_to, "Agent").cloned().unwrap();
                let move_path_empty = match agent.get("move_path") {
                    None => true,
                    Some(v) => v.as_array().map(|a| a.is_empty()).unwrap_or(true),
                };
                if move_path_empty {
                    if let Some(map) = &world.map {
                        if let Some(pathfinding) = map.find_path(&agent_cell, &target_cell) {
                            let move_path: Vec<JsonValue> = pathfinding
                                .path
                                .into_iter()
                                .skip(1)
                                .map(|cell| match cell {
                                    crate::map::CellKey::Square { x, y, z } => {
                                        serde_json::json!({ "Square": { "x": x, "y": y, "z": z } })
                                    }
                                    crate::map::CellKey::Hex { q, r, z } => {
                                        serde_json::json!({ "Hex": { "q": q, "r": r, "z": z } })
                                    }
                                    crate::map::CellKey::Region { ref id } => {
                                        serde_json::json!({ "Region": { "id": id } })
                                    }
                                })
                                .collect();
                            agent["move_path"] = serde_json::json!(move_path);
                            let _ = world.set_component(assigned_to, "Agent", agent);
                        }
                    }
                }
                job["phase"] = serde_json::json!("delivering_resources");
                return job;
            } else {
                let mut agent = world.get_component(assigned_to, "Agent").cloned().unwrap();
                let carried = agent
                    .get("carried_resources")
                    .cloned()
                    .unwrap_or(serde_json::json!([]));
                job["delivered_resources"] = carried.clone();
                job["phase"] = serde_json::json!("in_progress");
                agent.as_object_mut().unwrap().remove("carried_resources");
                let _ = world.set_component(assigned_to, "Agent", agent);
                return job;
            }
        }
    }
    job
}

fn handle_pathfinding_failure(world: &mut World, _eid: u32, mut job: JsonValue) -> JsonValue {
    // Mark job as blocked, emit event, unassign agent
    job["phase"] = serde_json::json!("blocked");
    job["status"] = serde_json::json!("blocked");
    if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64()) {
        let agent_id = agent_id as u32;
        if let Some(mut agent) = world.get_component(agent_id, "Agent").cloned() {
            agent.as_object_mut().unwrap().remove("current_job");
            agent["state"] = serde_json::json!("idle");
            let _ = world.set_component(agent_id, "Agent", agent);
        }
        job.as_object_mut().unwrap().remove("assigned_to");
    }
    // Emit job_blocked event
    crate::systems::job::system::JobSystem::emit_job_event(world, "job_blocked", &job, None);
    job
}
