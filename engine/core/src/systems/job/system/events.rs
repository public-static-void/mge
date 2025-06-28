//! Job event emission logic for the job system.

use crate::ecs::event_logger::EventLogger;
use crate::ecs::world::World;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::OnceLock;

// Global, thread-safe, lazily-initialized event logger
static JOB_EVENT_LOGGER: OnceLock<Arc<EventLogger<JsonValue>>> = OnceLock::new();

/// Initialize the job event logger (call at startup, only once).
pub fn init_job_event_logger() {
    JOB_EVENT_LOGGER.get_or_init(|| Arc::new(EventLogger::new()));
}

/// Get a reference to the global job event logger.
pub fn job_event_logger() -> Arc<EventLogger<JsonValue>> {
    JOB_EVENT_LOGGER
        .get()
        .expect("Job event logger not initialized")
        .clone()
}

/// Emits a job-related event to the world's event system and logs it.
/// The event payload includes the following fields (if present in the job):
/// - entity: The job's ID, if present in the job as "id"
/// - job_type: The job's type
/// - state: The job's state
/// - progress: The job's progress
/// - assigned_to: The entity assigned to the job
/// - priority: The job's priority
/// - Any extra fields passed in the extra map
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
    if let Some(priority) = job.get("priority") {
        payload.insert("priority".to_string(), priority.clone());
    }
    if let Some(extra) = extra {
        for (k, v) in extra {
            payload.insert(k.clone(), v.clone());
        }
    }
    let event_payload = JsonValue::Object(payload.clone());
    world.send_event(event, event_payload.clone()).ok();

    // Log the event
    job_event_logger().log(event, event_payload);
}

/// Save the job event log to a file.
pub fn save_job_event_log(path: &str) -> anyhow::Result<()> {
    job_event_logger().save_to_file(path)
}

/// Load a job event log from a file (overwrites current log).
pub fn load_job_event_log(path: &str) -> anyhow::Result<()> {
    let loaded_logger = EventLogger::load_from_file(path)?;
    let logger = job_event_logger();
    logger.clear();
    for event in loaded_logger.all() {
        logger.log(&event.event_type, event.payload);
    }
    Ok(())
}

/// Replay the job event log into the world's event system.
pub fn replay_job_event_log(world: &mut World) {
    job_event_logger().replay_into(|event| {
        world
            .send_event(&event.event_type, event.payload.clone())
            .ok();
    });
}
