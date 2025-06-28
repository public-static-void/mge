//! Job event emission logic for the job system.

use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// Emits a job-related event to the world's event system.
pub fn emit_job_event(
    world: &mut World,
    event: &str,
    job: &JsonValue,
    extra: Option<&serde_json::Map<String, JsonValue>>,
) {
    let mut payload = serde_json::Map::new();
    if let Some(id) = job.get("id") {
        payload.insert("entity".to_string(), id.clone());
    }
    if let Some(job_type) = job.get("job_type") {
        payload.insert("job_type".to_string(), job_type.clone());
    }
    if let Some(state) = job.get("state") {
        payload.insert("state".to_string(), state.clone());
    }
    if let Some(progress) = job.get("progress") {
        payload.insert("progress".to_string(), progress.clone());
    }
    if let Some(assigned_to) = job.get("assigned_to") {
        payload.insert("assigned_to".to_string(), assigned_to.clone());
    }
    if let Some(extra) = extra {
        for (k, v) in extra {
            payload.insert(k.clone(), v.clone());
        }
    }
    world.send_event(event, JsonValue::Object(payload)).ok();
}
