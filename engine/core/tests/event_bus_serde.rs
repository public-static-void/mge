use engine_core::ecs::event::EventBus;

#[test]
fn test_event_bus_serialize_deserialize() {
    let mut bus = EventBus::<i32>::default();
    bus.send(42);
    bus.send(100);
    bus.update(); // move events to last_events

    // Serialize using the custom method
    let ser = serde_json::to_string(&(&bus.last_events(), &bus.last_events())).unwrap();

    // Deserialize using the custom method
    let (events, last_events): (
        std::collections::VecDeque<i32>,
        std::collections::VecDeque<i32>,
    ) = serde_json::from_str(&ser).unwrap();
    let mut bus2 = EventBus::<i32>::default();
    bus2.set_events(events);
    bus2.set_last_events(last_events);

    // Subscribers are not serialized
    assert_eq!(bus2.subscriber_count(), 0);

    // Events and last_events are preserved
    assert_eq!(bus2.last_events(), bus.last_events());
}
