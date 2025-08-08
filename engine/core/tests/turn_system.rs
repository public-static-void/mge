#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::death_decay::{ProcessDeaths, ProcessDecay};
use serde_json::json;

#[test]
fn test_tick_advances_turn_and_runs_systems() {
    let mut world = make_test_world();
    world.current_mode = "colony".to_string();

    // Add map with both the initial and target cells
    let mut grid = SquareGridMap::new();
    grid.add_cell(1, 2, 0); // initial
    grid.add_cell(2, 2, 0); // after move
    world.map = Some(Map::new(Box::new(grid)));

    let id = world.spawn_entity();
    world
        .set_component(
            id,
            "Position",
            json!({ "pos": { "Square": { "x": 1, "y": 2, "z": 0 } } }),
        )
        .unwrap();
    world
        .set_component(id, "Health", json!({ "current": 10.0, "max": 10.0 }))
        .unwrap();

    // Move all: increment x for all entities with Position (Square)
    if let Some(positions) = world.components.get_mut("Position") {
        for (_eid, value) in positions.iter_mut() {
            if let Some(obj) = value.as_object_mut()
                && let Some(pos) = obj.get_mut("pos")
                && let Some(square) = pos.get_mut("Square")
                && let Some(x) = square.get_mut("x")
                && let Some(x_val) = x.as_i64()
            {
                *x = json!(x_val + 1);
            }
        }
    }
    // Damage all: decrement health for all entities with Health
    if let Some(healths) = world.components.get_mut("Health") {
        for (_eid, value) in healths.iter_mut() {
            if let Some(obj) = value.as_object_mut()
                && let Some(current) = obj.get_mut("current")
                && let Some(cur_val) = current.as_f64()
            {
                let new_val = (cur_val - 1.0).max(0.0);
                *current = json!(new_val);
            }
        }
    }
    world.register_system(ProcessDeaths);
    world.run_system("ProcessDeaths", None).unwrap();
    world.register_system(ProcessDecay);
    world.run_system("ProcessDecay", None).unwrap();
    world.turn += 1;

    // Position should be x+1, Health should be -1
    let pos = world.get_component(id, "Position").unwrap();
    let health = world.get_component(id, "Health").unwrap();

    assert!((pos["pos"]["Square"]["x"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    assert!((health["current"].as_f64().unwrap() - 9.0).abs() < 1e-6);
    assert_eq!(world.turn, 1);
}
