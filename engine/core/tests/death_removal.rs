#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::death_decay::{ProcessDeaths, ProcessDecay};
use serde_json::json;

#[test]
fn test_death_replaces_health_with_corpse_and_decay() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let id = world.spawn_entity();
    world
        .set_component(id, "Health", json!({ "current": 1.0, "max": 10.0 }))
        .unwrap();

    // Simulate damage that kills the entity
    if let Some(healths) = world.components.get_mut("Health") {
        for (_eid, value) in healths.iter_mut() {
            if let Some(obj) = value.as_object_mut() {
                if let Some(current) = obj.get_mut("current") {
                    if let Some(cur_val) = current.as_f64() {
                        // Subtract 2.0 damage
                        let new_val = (cur_val - 2.0).max(0.0);
                        *current = serde_json::json!(new_val);
                    }
                }
            }
        }
    }

    // Process deaths (to be implemented)
    world.register_system(ProcessDeaths);
    world.run_system("ProcessDeaths", None).unwrap();

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
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let id = world.spawn_entity();
    world.set_component(id, "Corpse", json!({})).unwrap();
    world
        .set_component(id, "Decay", json!({ "time_remaining": 2 }))
        .unwrap();

    // Tick 1
    world.register_system(ProcessDecay);
    world.run_system("ProcessDecay", None).unwrap();
    let decay = world.get_component(id, "Decay").unwrap();
    assert_eq!(decay["time_remaining"].as_u64().unwrap(), 1);

    // Tick 2 - entity should be removed
    world.register_system(ProcessDecay);
    world.run_system("ProcessDecay", None).unwrap();
    assert!(world.get_component(id, "Decay").is_none());
    // Optionally, check entity no longer exists (depends on your ECS API)
}
