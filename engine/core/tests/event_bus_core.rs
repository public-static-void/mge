use engine_core::ecs::event::EventBus;
use engine_core::ecs::event_bus_registry::EventBusRegistry;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
struct FooEvent(pub i32);

#[test]
fn test_event_bus_subscriptions() {
    let bus = Arc::new(Mutex::new(EventBus::<FooEvent>::default()));
    let mut registry = EventBusRegistry::new();
    registry.register_event_bus::<FooEvent>("foo".to_string(), bus.clone());

    let called = Arc::new(Mutex::new(Vec::new()));
    let called2 = called.clone();
    let sub_id = registry
        .subscribe::<FooEvent, _>("foo", move |event| {
            called2.lock().unwrap().push(event.0);
        })
        .unwrap();

    // Send event, handler should be called
    bus.lock().unwrap().send(FooEvent(42));
    assert_eq!(*called.lock().unwrap(), vec![42]);

    // Unsubscribe and send again, handler should not be called
    assert!(registry.unsubscribe::<FooEvent>("foo", sub_id));
    bus.lock().unwrap().send(FooEvent(99));
    assert_eq!(*called.lock().unwrap(), vec![42]);
}

#[test]
fn test_event_bus_subscribe_with_filter() {
    let mut bus = EventBus::<i32>::default();
    let called = Arc::new(Mutex::new(Vec::new()));

    let called1 = called.clone();
    bus.subscribe_with_filter(move |e| called1.lock().unwrap().push(*e), |e| *e % 2 == 0);

    bus.send(1);
    bus.send(2);
    bus.send(3);
    bus.send(4);

    assert_eq!(*called.lock().unwrap(), vec![2, 4]);
}

#[test]
fn test_event_bus_subscribe_with_map() {
    let mut bus = EventBus::<String>::default();
    let called = Arc::new(Mutex::new(Vec::new()));

    let called1 = called.clone();
    bus.subscribe_with_map(
        move |len| called1.lock().unwrap().push(len),
        |e| {
            if e.starts_with("foo") {
                Some(e.len())
            } else {
                None
            }
        },
    );

    bus.send("foo".into());
    bus.send("bar".into());
    bus.send("foobar".into());

    assert_eq!(*called.lock().unwrap(), vec![3, 6]);
}

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
