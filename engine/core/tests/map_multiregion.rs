#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::map::SquareGridMap;

#[test]
fn test_cells_in_multiple_regions() {
    // Use the shared helper to load schemas via config
    let mut world = world_helper::make_test_world();

    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    world.map = Some(engine_core::map::Map::new(Box::new(grid)));

    // Assign cell (0,0,0) to both "room_1" and "biome_A"
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "RegionAssignment",
            serde_json::json!({
                "cell": { "Square": { "x": 0, "y": 0, "z": 0 } },
                "region_id": ["room_1", "biome_A"]
            }),
        )
        .unwrap();

    // Assign cell (1,0,0) to just "room_1"
    let eid2 = world.spawn_entity();
    world
        .set_component(
            eid2,
            "RegionAssignment",
            serde_json::json!({
                "cell": { "Square": { "x": 1, "y": 0, "z": 0 } },
                "region_id": "room_1"
            }),
        )
        .unwrap();

    // Query all cells in "room_1"
    let cells_room = world.cells_in_region("room_1");
    assert!(cells_room.contains(&serde_json::json!({"Square": {"x": 0, "y": 0, "z": 0}})));
    assert!(cells_room.contains(&serde_json::json!({"Square": {"x": 1, "y": 0, "z": 0}})));

    // Query all cells in "biome_A"
    let cells_biome = world.cells_in_region("biome_A");
    assert!(cells_biome.contains(&serde_json::json!({"Square": {"x": 0, "y": 0, "z": 0}})));
    assert!(!cells_biome.contains(&serde_json::json!({"Square": {"x": 1, "y": 0, "z": 0}})));
}
