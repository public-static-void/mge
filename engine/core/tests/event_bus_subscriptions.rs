use engine_core::ecs::event::EventBus;
use engine_core::ecs::event_bus_registry::EventBusRegistry;
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
