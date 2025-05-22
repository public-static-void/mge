use engine_core::ecs::event::EventBus;
use std::sync::{Arc, Mutex};

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
