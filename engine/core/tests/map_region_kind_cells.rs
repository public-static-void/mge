#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::map::SquareGridMap;
use serde_json::json;

#[test]
fn test_cells_by_region_kind() {
    let mut world = make_test_world();

    // Setup map cells
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_cell(0, 1, 0);
    world.map = Some(engine_core::map::Map::new(Box::new(grid)));

    // Assign region assignments with kinds
    let cell_assignments = vec![
        (
            json!({"Square": {"x": 0, "y": 0, "z": 0}}),
            "room_1",
            "room",
        ),
        (
            json!({"Square": {"x": 1, "y": 0, "z": 0}}),
            "room_1",
            "room",
        ),
        (
            json!({"Square": {"x": 0, "y": 1, "z": 0}}),
            "stockpile_1",
            "stockpile",
        ),
    ];

    for (cell, region_id, kind) in cell_assignments {
        let eid = world.spawn_entity();
        world
            .set_component(
                eid,
                "RegionAssignment",
                json!({"cell": cell, "region_id": region_id, "kind": kind}),
            )
            .unwrap();
    }

    // Query cells by region kind "room"
    let room_cells = world.cells_in_region_kind("room");
    assert!(room_cells.contains(&json!({"Square": {"x": 0, "y": 0, "z": 0}})));
    assert!(room_cells.contains(&json!({"Square": {"x": 1, "y": 0, "z": 0}})));
    assert!(!room_cells.contains(&json!({"Square": {"x": 0, "y": 1, "z": 0}})));

    // Query cells by region kind "stockpile"
    let stockpile_cells = world.cells_in_region_kind("stockpile");
    assert!(stockpile_cells.contains(&json!({"Square": {"x": 0, "y": 1, "z": 0}})));
    assert!(!stockpile_cells.contains(&json!({"Square": {"x": 1, "y": 0, "z": 0}})));
}
