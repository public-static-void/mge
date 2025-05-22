use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_world_eventbus_integration() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    // Register event bus via World
    let bus = world.get_or_create_event_bus("TestBus");
    bus.lock().unwrap().send(json!({"foo": 1}));
    bus.lock().unwrap().update();
    let events = world.take_events("TestBus");
    assert_eq!(events, vec![json!({"foo": 1})]);

    // Hot-reload: update event bus via registry
    let new_bus = Arc::new(Mutex::new(engine_core::ecs::event::EventBus::default()));
    world
        .event_buses
        .update_event_bus("TestBus".to_string(), new_bus.clone())
        .unwrap();
    assert!(Arc::ptr_eq(
        &world.get_event_bus("TestBus").unwrap(),
        &new_bus
    ));

    // Unregister event bus via registry
    world.event_buses.unregister_event_bus("TestBus").unwrap();
    assert!(world.get_event_bus("TestBus").is_none());
}
