use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::map::SquareGridMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas")
}

#[test]
fn test_cells_in_region() {
    // Load config and schemas
    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schemas = load_schemas_from_dir_with_modes(schema_dir(), &config.allowed_modes)
        .expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry);

    // Add cells to map
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_cell(0, 1, 0);
    world.map = Some(engine_core::map::Map::new(Box::new(grid)));

    // Assign regions to cells using RegionAssignment component
    let cell_assignments = vec![
        (
            serde_json::json!({"Square": {"x": 0, "y": 0, "z": 0}}),
            "room_1",
        ),
        (
            serde_json::json!({"Square": {"x": 1, "y": 0, "z": 0}}),
            "room_1",
        ),
        (
            serde_json::json!({"Square": {"x": 0, "y": 1, "z": 0}}),
            "room_2",
        ),
    ];
    for (cell, region_id) in cell_assignments {
        let eid = world.spawn_entity();
        world
            .set_component(
                eid,
                "RegionAssignment",
                serde_json::json!({"cell": cell, "region_id": region_id}),
            )
            .unwrap();
    }

    // Query all cells in region "room_1"
    let cells = world.cells_in_region("room_1");
    assert!(cells.contains(&serde_json::json!({"Square": {"x": 0, "y": 0, "z": 0}})));
    assert!(cells.contains(&serde_json::json!({"Square": {"x": 1, "y": 0, "z": 0}})));
    assert!(!cells.contains(&serde_json::json!({"Square": {"x": 0, "y": 1, "z": 0}})));
}
