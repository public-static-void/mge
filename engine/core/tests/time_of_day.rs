use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn setup_world() -> Rc<RefCell<World>> {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    Rc::new(RefCell::new(World::new(registry)))
}

#[test]
fn test_time_of_day_initialization() {
    let world_rc = setup_world();
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
}

#[test]
fn test_time_of_day_advances_on_tick() {
    let world_rc = setup_world();
    World::tick(Rc::clone(&world_rc));
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.minute, 1);
}

#[test]
fn test_time_of_day_wraps_after_24_hours() {
    let world_rc = setup_world();
    for _ in 0..(24 * 60) {
        World::tick(Rc::clone(&world_rc));
    }
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
}
