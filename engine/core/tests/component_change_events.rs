#[path = "helpers/world.rs"]
mod world_helper;

use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_component_change_events() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let changes: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));

    // Ensure the event bus exists before subscribing
    world.get_or_create_event_bus::<serde_json::Value>("component_changed");

    // Subscribe to component change events
    let changes_sub = changes.clone();
    let sub_id = world
        .subscribe::<serde_json::Value, _>("component_changed", move |event| {
            changes_sub.lock().unwrap().push(event.clone());
        })
        .unwrap();

    // Set component (should trigger change event)
    // Use "Health" component, which is present in your schemas
    world
        .set_component(eid, "Health", json!({"current": 100, "max": 100}))
        .unwrap();
    world.update_event_buses::<serde_json::Value>();

    let changes_lock = changes.lock().unwrap();
    assert!(
        !changes_lock.is_empty(),
        "No component change event received"
    );
    let event = &changes_lock[0];
    assert_eq!(event["entity"], eid);
    assert_eq!(event["component"], "Health");
    assert_eq!(event["action"], "set");
    assert_eq!(event["new"]["current"], 100);
    assert_eq!(event["new"]["max"], 100);
    drop(changes_lock);

    // Remove component (should trigger change event)
    world.remove_component(eid, "Health").unwrap();
    world.update_event_buses::<serde_json::Value>();
    let changes_lock = changes.lock().unwrap();
    assert!(changes_lock.iter().any(|e| e["action"] == "removed"));
    drop(changes_lock);

    // Unsubscribe and ensure no further events are received
    world.unsubscribe::<serde_json::Value>("component_changed", sub_id);
    world
        .set_component(eid, "Health", json!({"current": 50, "max": 100}))
        .unwrap();
    world.update_event_buses::<serde_json::Value>();
    let changes_lock = changes.lock().unwrap();
    let set_count = changes_lock.iter().filter(|e| e["action"] == "set").count();
    assert_eq!(
        set_count, 1,
        "Should not receive new events after unsubscribe"
    );
}
