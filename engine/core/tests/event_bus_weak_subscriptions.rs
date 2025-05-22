use engine_core::ecs::event::EventBus;
use std::sync::{Arc, Mutex};

#[test]
fn test_weak_subscription_auto_unsubscribe() {
    let mut bus = EventBus::<i32>::default();
    let called = Arc::new(Mutex::new(Vec::new()));

    let owner = Arc::new(());
    let called1 = called.clone();

    bus.subscribe_weak(&owner, move |e| {
        called1.lock().unwrap().push(*e);
    });

    bus.send(1);
    assert_eq!(*called.lock().unwrap(), vec![1]);
    assert_eq!(bus.subscriber_count(), 1);

    // Drop owner, should auto-unsubscribe
    drop(owner);

    bus.send(2);
    assert_eq!(*called.lock().unwrap(), vec![1]);
    assert_eq!(bus.subscriber_count(), 0);
}
