use engine_core::ecs::event::{EventBus, EventReader};

#[derive(Clone, Debug, PartialEq)]
struct TestEvent {
    pub value: i32,
}

#[test]
fn test_event_send_and_receive() {
    let mut bus = EventBus::<TestEvent>::default();
    let mut reader = EventReader::new();

    // No events yet
    assert_eq!(reader.read(&bus).count(), 0);

    // Send an event
    bus.send(TestEvent { value: 42 });

    // Advance the event bus to make the event visible to readers
    bus.update();

    // Should receive the event
    let events: Vec<_> = reader.read(&bus).cloned().collect();
    assert_eq!(events, vec![TestEvent { value: 42 }]);

    // Reading again should yield nothing (event consumed)
    assert_eq!(reader.read(&bus).count(), 0);
}
