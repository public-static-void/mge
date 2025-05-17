use engine_core::ecs::registry::ComponentRegistry;
use engine_core::scripting::world::World;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_dynamic_event_bus_send_and_read() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    // Send FooEvent (as JSON)
    let foo_bus = world.get_or_create_event_bus("foo_event");
    foo_bus.lock().unwrap().send(json!({ "value": 123 }));

    // Send BarEvent (as JSON)
    let bar_bus = world.get_or_create_event_bus("bar_event");
    bar_bus.lock().unwrap().send(json!({ "value": "hello" }));

    // Advance event buses
    world.update_event_buses();

    // Read FooEvent
    let mut foo_reader = engine_core::ecs::event::EventReader::default();
    let foo_events: Vec<_> = foo_reader
        .read(&*foo_bus.lock().unwrap())
        .cloned()
        .collect();
    assert_eq!(foo_events, vec![json!({ "value": 123 })]);

    // Read BarEvent
    let mut bar_reader = engine_core::ecs::event::EventReader::default();
    let bar_events: Vec<_> = bar_reader
        .read(&*bar_bus.lock().unwrap())
        .cloned()
        .collect();
    assert_eq!(bar_events, vec![json!({ "value": "hello" })]);
}
