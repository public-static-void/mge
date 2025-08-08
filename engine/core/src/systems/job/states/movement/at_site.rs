//! Handler for the "at_site" job state.

use crate::ecs::world::World;
use crate::systems::job::are_requirements_met;
use serde_json::{Value as JsonValue, json};

/// Handles the "at_site" state.
///
/// If the job has resource requirements still needed to deliver,
/// transition to "fetching_resources",
/// otherwise transition to "in_progress".
pub fn handle_at_site_state(world: &mut World, eid: u32, mut job: JsonValue) -> JsonValue {
    if job.get("assigned_to").and_then(|v| v.as_u64()).is_none() {
        return job;
    }

    let requirements = job
        .get("resource_requirements")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let delivered_resources = job
        .get("delivered_resources")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if !are_requirements_met(&requirements, &delivered_resources) {
        job["state"] = json!("fetching_resources");
    } else {
        job["state"] = json!("in_progress");
    }

    world.set_component(eid, "Job", job.clone()).unwrap();
    job
}
