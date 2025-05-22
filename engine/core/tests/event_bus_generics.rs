use engine_core::ecs::event::{EventBus, EventReader};
use engine_core::ecs::event_bus_registry::EventBusRegistry;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
struct FooEvent(pub i32);

#[derive(Debug, Clone, PartialEq)]
struct BarEvent(pub String);

#[test]
fn test_register_and_get_type_safe_event_buses() {
    let mut registry = EventBusRegistry::new();

    // Register FooEvent bus
    let foo_bus = Arc::new(Mutex::new(EventBus::<FooEvent>::default()));
    registry.register_event_bus::<FooEvent>("foo".to_string(), foo_bus.clone());

    // Register BarEvent bus
    let bar_bus = Arc::new(Mutex::new(EventBus::<BarEvent>::default()));
    registry.register_event_bus::<BarEvent>("bar".to_string(), bar_bus.clone());

    // Send and read FooEvent
    foo_bus.lock().unwrap().send(FooEvent(42));
    foo_bus.lock().unwrap().update();
    let mut foo_reader = EventReader::default();
    let foo_events: Vec<_> = foo_reader
        .read(&*foo_bus.lock().unwrap())
        .cloned()
        .collect();
    assert_eq!(foo_events, vec![FooEvent(42)]);

    // Send and read BarEvent
    bar_bus.lock().unwrap().send(BarEvent("hello".to_string()));
    bar_bus.lock().unwrap().update();
    let mut bar_reader = EventReader::default();
    let bar_events: Vec<_> = bar_reader
        .read(&*bar_bus.lock().unwrap())
        .cloned()
        .collect();
    assert_eq!(bar_events, vec![BarEvent("hello".to_string())]);

    // Type-safe retrieval
    let foo_bus_2 = registry.get_event_bus::<FooEvent>("foo").unwrap();
    assert!(Arc::ptr_eq(&foo_bus, &foo_bus_2));
    let bar_bus_2 = registry.get_event_bus::<BarEvent>("bar").unwrap();
    assert!(Arc::ptr_eq(&bar_bus, &bar_bus_2));

    // Type mismatch should return None
    assert!(registry.get_event_bus::<FooEvent>("bar").is_none());
    assert!(registry.get_event_bus::<BarEvent>("foo").is_none());

    // Unregister
    assert!(registry.unregister_event_bus::<FooEvent>("foo"));
    assert!(!registry.unregister_event_bus::<FooEvent>("foo")); // Already removed
}
