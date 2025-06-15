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
