use engine_core::ecs::event::{EventBus, EventReader};
use engine_core::ecs::event_bus_registry::EventBusRegistry;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_event_bus_register_update_unregister() {
    let mut registry = EventBusRegistry::new();

    // Register initial event bus
    let bus_name = "TestBus".to_string();
    let bus = Arc::new(Mutex::new(EventBus::default()));
    registry.register_event_bus::<serde_json::Value>(bus_name.clone(), bus.clone());

    // Send event and verify
    bus.lock().unwrap().send(json!({"value": 42}));
    bus.lock().unwrap().update();
    let mut reader = EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
    assert_eq!(events, vec![json!({"value": 42})]);

    // Hot-reload: just re-register with the same name/type
    let new_bus = Arc::new(Mutex::new(EventBus::default()));
    registry.register_event_bus::<serde_json::Value>(bus_name.clone(), new_bus.clone());

    // After update, old bus is replaced
    assert!(Arc::ptr_eq(
        &registry
            .get_event_bus::<serde_json::Value>(&bus_name)
            .unwrap(),
        &new_bus
    ));

    // Unregister event bus
    assert!(registry.unregister_event_bus::<serde_json::Value>(&bus_name));
    assert!(
        registry
            .get_event_bus::<serde_json::Value>(&bus_name)
            .is_none()
    );
}
