use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::Season;
use engine_core::ecs::world::World;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn test_time_of_day_initialization() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
    assert_eq!(time.day, 0);
}

#[test]
fn test_time_of_day_advances_on_tick() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    World::tick(Rc::clone(&world_rc));
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.minute, 1);
}

#[test]
fn test_time_of_day_wraps_after_24_hours() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    for _ in 0..(24 * 60) {
        World::tick(Rc::clone(&world_rc));
    }
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
    assert_eq!(time.day, 1);
}

#[test]
fn test_day_increments_after_full_day() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    for _ in 0..(24 * 60) {
        World::tick(Rc::clone(&world_rc));
    }
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.day, 1);
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
}

#[test]
fn test_season_default_is_spring() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(Season::from_day(time.day), Season::Spring);
}

#[test]
fn test_season_from_day_boundaries() {
    assert_eq!(Season::from_day(0), Season::Spring);
    assert_eq!(Season::from_day(29), Season::Spring);
    assert_eq!(Season::from_day(30), Season::Summer);
    assert_eq!(Season::from_day(59), Season::Summer);
    assert_eq!(Season::from_day(60), Season::Autumn);
    assert_eq!(Season::from_day(89), Season::Autumn);
    assert_eq!(Season::from_day(90), Season::Winter);
    assert_eq!(Season::from_day(119), Season::Winter);
    assert_eq!(Season::from_day(120), Season::Spring);
    assert_eq!(Season::from_day(240), Season::Spring);
    assert_eq!(Season::from_day(555), Season::Autumn); // 555 % 120 = 75, 75/30 = 2 -> Autumn
}

#[test]
fn test_season_display_lowercase() {
    assert_eq!(Season::Spring.to_string(), "spring");
    assert_eq!(Season::Summer.to_string(), "summer");
    assert_eq!(Season::Autumn.to_string(), "autumn");
    assert_eq!(Season::Winter.to_string(), "winter");
}
