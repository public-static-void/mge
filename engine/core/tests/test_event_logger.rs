use engine_core::ecs::event_logger::EventLogger;
use serde_json::json;
use std::fs;

#[test]
fn test_event_logger_log_and_replay() {
    let logger = EventLogger::new();

    logger.log("job_assigned", json!({"entity": 1, "state": "in_progress"}));
    logger.log("job_completed", json!({"entity": 1, "state": "complete"}));

    let events = logger.all();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "job_assigned");
    assert_eq!(events[1].event_type, "job_completed");

    let mut replayed = Vec::new();
    logger.replay_into(|e| replayed.push(e.event_type.clone()));
    assert_eq!(replayed, vec!["job_assigned", "job_completed"]);
}

#[test]
fn test_event_logger_save_and_load() {
    let logger = EventLogger::new();
    logger.log("job_failed", json!({"entity": 2, "state": "failed"}));

    let path = "test_event_log.json";
    logger.save_to_file(path).unwrap();

    let loaded: EventLogger<serde_json::Value> = EventLogger::load_from_file(path).unwrap();
    fs::remove_file(path).unwrap();

    let events = loaded.all();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "job_failed");
    assert_eq!(events[0].payload["state"], "failed");
}
