#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::map::SquareGridMap;
use serde_json::json;

#[test]
fn test_entities_in_region() {
    // Use the shared helper to load schemas via config
    let mut world = world_helper::make_test_world();

    // Add cells and entities
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    world.map = Some(engine_core::map::Map::new(Box::new(grid)));

    let eid1 = world.spawn_entity();
    let eid2 = world.spawn_entity();
    let eid3 = world.spawn_entity();

    // Assign positions and regions
    world
        .set_component(
            eid1,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(
            eid2,
            "Position",
            json!({"pos": {"Square": {"x": 1, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(
            eid3,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();

    // Assign regions
    world
        .set_component(eid1, "Region", json!({"id": "room_1", "kind": "room"}))
        .unwrap();
    world
        .set_component(eid2, "Region", json!({"id": "room_2", "kind": "room"}))
        .unwrap();
    world
        .set_component(eid3, "Region", json!({"id": "room_1", "kind": "room"}))
        .unwrap();

    // Query all entities in region "room_1"
    let entities = world.entities_in_region("room_1");
    assert!(entities.contains(&eid1), "eid1 should be in room_1");
    assert!(entities.contains(&eid3), "eid3 should be in room_1");
    assert!(!entities.contains(&eid2), "eid2 should not be in room_1");
}
