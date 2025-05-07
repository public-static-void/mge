use engine_core::scripting::World;
use serde_json::json;

#[test]
fn test_death_replaces_health_with_corpse_and_decay() {
    let mut world = World::new();

    let id = world.spawn();
    world
        .set_component(id, "Health", json!({ "current": 1.0, "max": 10.0 }))
        .unwrap();

    // Simulate damage that kills the entity
    world.damage_all(2.0);

    // Process deaths (to be implemented)
    world.process_deaths();

    // Health component should be removed
    assert!(world.get_component(id, "Health").is_none());

    // Corpse component should be present
    assert!(world.get_component(id, "Corpse").is_some());

    // Decay component should be present with default time_remaining
    let decay = world.get_component(id, "Decay").unwrap();
    assert_eq!(decay["time_remaining"].as_u64().unwrap(), 5);
}

#[test]
fn test_decay_removes_entity_after_time() {
    let mut world = World::new();

    let id = world.spawn();
    world.set_component(id, "Corpse", json!({})).unwrap();
    world
        .set_component(id, "Decay", json!({ "time_remaining": 2 }))
        .unwrap();

    // Tick 1
    world.process_decay();
    let decay = world.get_component(id, "Decay").unwrap();
    assert_eq!(decay["time_remaining"].as_u64().unwrap(), 1);

    // Tick 2 - entity should be removed
    world.process_decay();
    assert!(world.get_component(id, "Decay").is_none());
    // Optionally, check entity no longer exists (depends on your ECS API)
}
