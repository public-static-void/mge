use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// Processes all child jobs of a parent job, updating their states and propagating cancellation if needed.
/// Returns a tuple: (updated children array, all_children_complete: bool)
pub fn process_job_children(
    world: &mut World,
    lua: Option<&mlua::Lua>,
    eid: u32,
    children_val: &mut JsonValue,
    parent_is_cancelled: bool,
) -> (JsonValue, bool) {
    let children = std::mem::take(children_val)
        .as_array_mut()
        .map(std::mem::take)
        .unwrap_or_default();

    // Collect child IDs for ECS access
    let mut child_ids = Vec::new();
    for child in &children {
        if let Some(child_id) = child.get("id").and_then(|v| v.as_u64()) {
            child_ids.push(child_id as u32);
            if parent_is_cancelled {
                // Set ECS state to cancelled and process until terminal
                let mut child_job = world.get_component(child_id as u32, "Job").unwrap().clone();
                child_job["state"] = JsonValue::from("cancelled");
                world
                    .set_component(child_id as u32, "Job", child_job.clone())
                    .unwrap();
                let mut passes = 0;
                loop {
                    let child_job = world.get_component(child_id as u32, "Job").unwrap().clone();
                    let state = child_job
                        .get("state")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if matches!(state, "complete" | "failed" | "cancelled" | "interrupted") {
                        break;
                    }
                    passes += 1;
                    if passes > 16 {
                        panic!(
                            "Child job {child_id} stuck in non-terminal state {state:?} after 16 passes: {child_job:?}"
                        );
                    }
                    let mut next_child = child_job.clone();
                    next_child["state"] = JsonValue::from("cancelled");
                    let processed_child = crate::systems::job::system::process::process_job(
                        world,
                        lua,
                        child_id as u32,
                        next_child,
                    );
                    world
                        .set_component(child_id as u32, "Job", processed_child.clone())
                        .unwrap();
                }
            } else {
                // Normal processing
                let processed = crate::systems::job::system::process::process_job(
                    world,
                    lua,
                    eid,
                    child.clone(),
                );
                world
                    .set_component(child_id as u32, "Job", processed.clone())
                    .unwrap();
            }
        }
    }

    // Always reconstruct the children array from ECS after processing
    let updated_children: Vec<JsonValue> = child_ids
        .iter()
        .map(|&child_id| world.get_component(child_id, "Job").unwrap().clone())
        .collect();

    let all_children_complete = !parent_is_cancelled
        && !updated_children.is_empty()
        && updated_children
            .iter()
            .all(|c| c.get("state").and_then(|v| v.as_str()) == Some("complete"));

    (JsonValue::Array(updated_children), all_children_complete)
}
