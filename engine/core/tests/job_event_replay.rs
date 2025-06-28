use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::systems::job::system::events::{
    emit_job_event, init_job_event_logger, load_job_event_log, replay_job_event_log,
    save_job_event_log,
};
use serde_json::json;
use std::fs;
use std::sync::{Arc, Mutex};

#[test]
fn test_job_event_logging_and_replay() {
    // Clean up any previous test log
    let log_path = "test_job_event_log.json";
    let _ = fs::remove_file(log_path);

    // --- Original run: emit events and save log ---
    let registry = Arc::new(Mutex::new(ComponentRegistry::default()));
    init_job_event_logger();
    let mut world = World::new(registry.clone());

    // Create a dummy job
    let job = json!({
        "id": 42,
        "job_type": "dig",
        "state": "pending",
        "priority": 5,
        "assigned_to": null
    });

    // Subscribe to job events and collect them
    let received_events = Arc::new(Mutex::new(Vec::new()));
    {
        let received_events = received_events.clone();
        world
            .event_buses
            .get_event_bus::<serde_json::Value>("job_assigned")
            .unwrap_or_else(|| {
                world.event_buses.register_event_bus(
                    "job_assigned".to_string(),
                    Arc::new(Mutex::new(engine_core::ecs::event::EventBus::<
                        serde_json::Value,
                    >::default())),
                );
                world
                    .event_buses
                    .get_event_bus::<serde_json::Value>("job_assigned")
                    .unwrap()
            })
            .lock()
            .unwrap()
            .subscribe(move |event| {
                received_events.lock().unwrap().push(event.clone());
            });
    }

    // Emit a job assignment event
    emit_job_event(&mut world, "job_assigned", &job, None);

    // Save the event log
    save_job_event_log(log_path).expect("Failed to save job event log");

    // --- Replay run: load log and replay into a new world ---
    let registry = Arc::new(Mutex::new(ComponentRegistry::default()));
    init_job_event_logger();
    let mut replayed_world = World::new(registry);

    // Set up a new event bus and collector for replayed events
    let replayed_events = Arc::new(Mutex::new(Vec::new()));
    {
        let replayed_events = replayed_events.clone();
        replayed_world.event_buses.register_event_bus(
            "job_assigned".to_string(),
            Arc::new(Mutex::new(engine_core::ecs::event::EventBus::<
                serde_json::Value,
            >::default())),
        );
        replayed_world
            .event_buses
            .get_event_bus::<serde_json::Value>("job_assigned")
            .unwrap()
            .lock()
            .unwrap()
            .subscribe(move |event| {
                replayed_events.lock().unwrap().push(event.clone());
            });
    }

    // Load and replay the log
    load_job_event_log(log_path).expect("Failed to load job event log");
    replay_job_event_log(&mut replayed_world);

    // --- Assert that the replayed event matches the original ---
    let replayed_events = replayed_events.lock().unwrap();
    assert_eq!(replayed_events.len(), 1);
    assert_eq!(replayed_events[0]["entity"], 42);
    assert_eq!(replayed_events[0]["job_type"], "dig");
    assert_eq!(replayed_events[0]["state"], "pending");
    assert_eq!(replayed_events[0]["priority"], 5);
    assert!(replayed_events[0]["assigned_to"].is_null());

    // Clean up
    let _ = fs::remove_file(log_path);
}
