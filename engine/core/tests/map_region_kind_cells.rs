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

    // Region records carry the kind; assignments reference regions by id only.
    for (id, kind) in [("room_1", "room"), ("stockpile_1", "stockpile")] {
        let eid = world.spawn_entity();
        world
            .set_component(eid, "Region", json!({"id": id, "kind": kind}))
            .unwrap();
    }

    // Assign cells to regions (no `kind` on assignments — the schema has none).
    let cell_assignments = vec![
        (json!({"Square": {"x": 0, "y": 0, "z": 0}}), "room_1"),
        (json!({"Square": {"x": 1, "y": 0, "z": 0}}), "room_1"),
        (json!({"Square": {"x": 0, "y": 1, "z": 0}}), "stockpile_1"),
    ];

    for (cell, region_id) in cell_assignments {
        let eid = world.spawn_entity();
        world
            .set_component(
                eid,
                "RegionAssignment",
                json!({"cell": cell, "region_id": region_id}),
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

    // Unknown kind resolves to no cells.
    assert!(world.cells_in_region_kind("no_such_kind").is_empty());
}
