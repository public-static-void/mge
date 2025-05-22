use engine_core::ecs::event::EventBus;
use std::sync::{Arc, Mutex};

#[test]
fn test_subscribe_once() {
    let mut bus = EventBus::<i32>::default();
    let called = Arc::new(Mutex::new(Vec::new()));
    let called1 = called.clone();

    bus.subscribe_once(move |e| {
        called1.lock().unwrap().push(*e);
    });

    bus.send(10);
    bus.send(20);
    bus.send(30);

    // Only the first event should be handled
    assert_eq!(*called.lock().unwrap(), vec![10]);
    assert_eq!(bus.subscriber_count(), 0);
}
