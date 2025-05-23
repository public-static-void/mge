use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::map::SquareGridMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas")
}

#[test]
fn test_cells_by_region_kind() {
    let schemas = load_schemas_from_dir(schema_dir()).unwrap();
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry);

    // Setup map cells
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_cell(0, 1, 0);
    world.map = Some(engine_core::map::Map::new(Box::new(grid)));

    // Assign region assignments with kinds
    let cell_assignments = vec![
        (
            serde_json::json!({"Square": {"x": 0, "y": 0, "z": 0}}),
            "room_1",
            "room",
        ),
        (
            serde_json::json!({"Square": {"x": 1, "y": 0, "z": 0}}),
            "room_1",
            "room",
        ),
        (
            serde_json::json!({"Square": {"x": 0, "y": 1, "z": 0}}),
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
                serde_json::json!({"cell": cell, "region_id": region_id, "kind": kind}),
            )
            .unwrap();
    }

    // Query cells by region kind "room"
    let room_cells = world.cells_in_region_kind("room");
    assert!(room_cells.contains(&serde_json::json!({"Square": {"x": 0, "y": 0, "z": 0}})));
    assert!(room_cells.contains(&serde_json::json!({"Square": {"x": 1, "y": 0, "z": 0}})));
    assert!(!room_cells.contains(&serde_json::json!({"Square": {"x": 0, "y": 1, "z": 0}})));

    // Query cells by region kind "stockpile"
    let stockpile_cells = world.cells_in_region_kind("stockpile");
    assert!(stockpile_cells.contains(&serde_json::json!({"Square": {"x": 0, "y": 1, "z": 0}})));
    assert!(!stockpile_cells.contains(&serde_json::json!({"Square": {"x": 1, "y": 0, "z": 0}})));
}
