#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::system::events::{
    emit_job_event, init_job_event_logger, job_event_logger,
};
use serde_json::json;

#[test]
fn test_job_event_logging_integration() {
    init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Emit a job event
    emit_job_event(&mut world, "job_completed", &json!({"id": 1}), None);

    // Query the event log
    let events = job_event_logger().get_events_by_type("job_completed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload["entity"], 1);
}
