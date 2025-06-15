#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::systems::death_decay::ProcessDeaths;
use serde_json::json;

#[test]
fn test_move_all_system_moves_entities() {
    let mut world = make_test_world();

    // Spawn entities and assign Position components (using the schema-defined structure)
    for _ in 0..3 {
        let eid = world.spawn_entity();
        world
            .set_component(
                eid,
                "Position",
                json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
            )
            .expect("Failed to set Position component (check your schema)");
    }

    // Get all entity IDs
    let entities = world.get_entities();

    // Move all: increment x, y for all entities with Position
    if let Some(positions) = world.components.get_mut("Position") {
        for (_eid, value) in positions.iter_mut() {
            if let Some(obj) = value.as_object_mut() {
                if let Some(pos) = obj.get_mut("pos") {
                    if let Some(square) = pos.as_object_mut().unwrap().get_mut("Square") {
                        let square_obj = square.as_object_mut().unwrap();
                        if let Some(x) = square_obj.get_mut("x") {
                            if let Some(x_val) = x.as_i64() {
                                *x = json!(x_val + 1);
                            }
                        }
                        if let Some(y) = square_obj.get_mut("y") {
                            if let Some(y_val) = y.as_i64() {
                                *y = json!(y_val + 2);
                            }
                        }
                    }
                }
            }
        }
    }

    // Assert positions incremented
    for &eid in entities.iter().take(3) {
        let pos = world.get_component(eid, "Position").unwrap();
        let square = pos["pos"]["Square"].as_object().unwrap();
        let x = square["x"].as_i64().unwrap();
        let y = square["y"].as_i64().unwrap();
        assert_eq!(x, 1, "x should be incremented by 1");
        assert_eq!(y, 2, "y should be incremented by 2");
    }
}

#[test]
fn test_process_deaths_creates_corpse_and_decay() {
    let mut world = make_test_world();

    // Spawn entity and assign Health component
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Health", json!({"current": 10, "max": 10}))
        .unwrap();

    // Register death system
    world.register_system(ProcessDeaths);

    // Set health to 0 to trigger death
    world
        .set_component(eid, "Health", json!({"current": 0, "max": 10}))
        .unwrap();

    // Run death system
    world.run_system("ProcessDeaths", None).unwrap();

    // Assert Corpse and Decay components present
    let corpse = world.get_component(eid, "Corpse");
    assert!(
        corpse.is_some(),
        "Corpse component should be present after death"
    );

    let decay = world.get_component(eid, "Decay");
    assert!(
        decay.is_some(),
        "Decay component should be present after death"
    );
}
