use engine_core::ecs::event::EventBus;
use engine_core::ecs::event_bus_registry::EventBusRegistry;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};

#[test]
fn test_event_bus_introspection() {
    let mut registry = EventBusRegistry::new();

    let bus1 = Arc::new(Mutex::new(EventBus::<JsonValue>::default()));
    let bus2 = Arc::new(Mutex::new(EventBus::<JsonValue>::default()));
    registry.register_event_bus::<JsonValue>("bus1".to_string(), bus1.clone());
    registry.register_event_bus::<JsonValue>("bus2".to_string(), bus2.clone());

    // Subscribe to bus1
    let _id = registry.subscribe::<JsonValue, _>("bus1", |_event| {});

    // List buses
    let infos = registry.list_buses();
    let names: Vec<_> = infos.iter().map(|info| info.name.clone()).collect();
    assert!(names.contains(&"bus1".to_string()));
    assert!(names.contains(&"bus2".to_string()));

    // Check subscriber count
    let bus1_info = infos.iter().find(|info| info.name == "bus1").unwrap();
    assert_eq!(bus1_info.subscriber_count, 1);

    // List names only
    let name_list = registry.list_bus_names();
    assert!(name_list.contains(&"bus1".to_string()));
    assert!(name_list.contains(&"bus2".to_string()));
}
