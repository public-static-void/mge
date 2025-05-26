use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use std::sync::{Arc, Mutex};

fn setup_world() -> World {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    World::new(registry)
}

#[test]
fn test_time_of_day_initialization() {
    let world = setup_world();
    let time = world.get_time_of_day();
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
}

#[test]
fn test_time_of_day_advances_on_tick() {
    let mut world = setup_world();
    world.tick();
    let time = world.get_time_of_day();
    assert_eq!(time.minute, 1);
}

#[test]
fn test_time_of_day_wraps_after_24_hours() {
    let mut world = setup_world();
    for _ in 0..(24 * 60) {
        world.tick();
    }
    let time = world.get_time_of_day();
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
}
