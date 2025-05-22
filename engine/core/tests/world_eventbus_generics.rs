use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
struct FooEvent(pub i32);

#[test]
fn test_world_type_safe_event_buses() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    let bus = world.get_or_create_event_bus::<FooEvent>("foo");
    bus.lock().unwrap().send(FooEvent(123));
    bus.lock().unwrap().update();

    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
    assert_eq!(events, vec![FooEvent(123)]);

    // Update all FooEvent buses in the world
    world.update_event_buses::<FooEvent>();
}
