use engine_core::ecs::event_logger::EventLogger;
use serde_json::json;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_event_logger_query_by_type() {
    let logger = EventLogger::<serde_json::Value>::new();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));
    logger.log("job_completed", json!({"id": 3}));

    let events = logger.query_events(|e| e.event_type == "job_completed");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].payload["id"], 1);
    assert_eq!(events[1].payload["id"], 3);
}

#[test]
fn test_event_logger_query_by_time_range() {
    let logger = EventLogger::<serde_json::Value>::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));

    let events = logger.query_events(|e| e.timestamp >= now);
    assert_eq!(events.len(), 2);
}

#[test]
fn test_event_logger_query_by_payload_field() {
    let logger = EventLogger::<serde_json::Value>::new();
    logger.log("job_completed", json!({"id": 1, "priority": 10}));
    logger.log("job_failed", json!({"id": 2, "priority": 1}));
    logger.log("job_completed", json!({"id": 3, "priority": 10}));

    let events =
        logger.query_events(|e| e.payload.get("priority").and_then(|v| v.as_i64()) == Some(10));
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].payload["id"], 1);
    assert_eq!(events[1].payload["id"], 3);
}

#[test]
fn test_get_events_by_type() {
    let logger = EventLogger::<serde_json::Value>::new();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));
    logger.log("job_completed", json!({"id": 3}));

    let events = logger.get_events_by_type("job_completed");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].payload["id"], 1);
    assert_eq!(events[1].payload["id"], 3);
}

#[test]
fn test_get_events_since() {
    let logger = EventLogger::<serde_json::Value>::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));

    let events = logger.get_events_since(now);
    assert_eq!(events.len(), 2);
}

#[test]
fn test_get_events_where() {
    let logger = EventLogger::<serde_json::Value>::new();
    logger.log("job_completed", json!({"id": 1, "priority": 10}));
    logger.log("job_failed", json!({"id": 2, "priority": 1}));
    logger.log("job_completed", json!({"id": 3, "priority": 10}));

    let events =
        logger.get_events_where(|p| p.get("priority").and_then(|v| v.as_i64()) == Some(10));
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].payload["id"], 1);
    assert_eq!(events[1].payload["id"], 3);
}

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
