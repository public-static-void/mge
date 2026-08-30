use engine_core::map::SquareGridMap;
use serde_json::json;

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

#[test]
fn test_cells_in_region() {
    let mut world = make_test_world();

    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_cell(0, 1, 0);
    world.map = Some(engine_core::map::Map::new(Box::new(grid)));

    let cell_assignments = vec![
        (json!({"Square": {"x": 0, "y": 0, "z": 0}}), "room_1"),
        (json!({"Square": {"x": 1, "y": 0, "z": 0}}), "room_1"),
        (json!({"Square": {"x": 0, "y": 1, "z": 0}}), "room_2"),
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

    let cells = world.cells_in_region("room_1");
    assert!(cells.contains(&json!({"Square": {"x": 0, "y": 0, "z": 0}})));
    assert!(cells.contains(&json!({"Square": {"x": 1, "y": 0, "z": 0}})));
    assert!(!cells.contains(&json!({"Square": {"x": 0, "y": 1, "z": 0}})));
}

// A RegionAssignment whose cell is { "Region": { "id": R } } resolves to the
// actual cells of region R, not the opaque Region JSON.
#[test]
fn test_cells_in_region_resolves_region_cell() {
    let mut world = make_test_world();

    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    world.map = Some(engine_core::map::Map::new(Box::new(grid)));

    // Actual cells of region "R".
    let e1 = world.spawn_entity();
    world
        .set_component(
            e1,
            "RegionAssignment",
            json!({"cell": {"Square": {"x": 0, "y": 0, "z": 0}}, "region_id": "R"}),
        )
        .unwrap();
    let e2 = world.spawn_entity();
    world
        .set_component(
            e2,
            "RegionAssignment",
            json!({"cell": {"Square": {"x": 1, "y": 0, "z": 0}}, "region_id": "R"}),
        )
        .unwrap();

    // "outer" is region-anchored: its cell is the opaque Region JSON for R.
    let e3 = world.spawn_entity();
    world
        .set_component(
            e3,
            "RegionAssignment",
            json!({"cell": {"Region": {"id": "R"}}, "region_id": "outer"}),
        )
        .unwrap();

    let cells = world.cells_in_region("outer");
    assert!(cells.contains(&json!({"Square": {"x": 0, "y": 0, "z": 0}})));
    assert!(cells.contains(&json!({"Square": {"x": 1, "y": 0, "z": 0}})));
    assert!(!cells.contains(&json!({"Region": {"id": "R"}})));
    assert_eq!(cells.len(), 2);
}

// Region-reference cycles terminate instead of recursing forever.
#[test]
fn test_cells_in_region_cycle_guard() {
    let mut world = make_test_world();

    // Region A references region B; region B references region A (cycle).
    let e1 = world.spawn_entity();
    world
        .set_component(
            e1,
            "RegionAssignment",
            json!({"cell": {"Region": {"id": "B"}}, "region_id": "A"}),
        )
        .unwrap();
    let e2 = world.spawn_entity();
    world
        .set_component(
            e2,
            "RegionAssignment",
            json!({"cell": {"Region": {"id": "A"}}, "region_id": "B"}),
        )
        .unwrap();

    // No infinite loop; the cycle resolves to no concrete cells.
    let cells = world.cells_in_region("A");
    assert!(cells.is_empty());
}
