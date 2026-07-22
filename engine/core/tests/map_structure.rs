#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::{load_allowed_modes, load_schemas_from_dir_with_modes};
use engine_core::ecs::world::World;
use engine_core::map::{CellKey, MapTopology, SquareGridMap};
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use world_helper::make_test_world;

#[test]
fn test_multiz_cell_neighbors() {
    let mut grid = SquareGridMap::new();
    // Add cells at (0,0,0), (0,1,0), (0,0,1)
    grid.add_cell(0, 0, 0);
    grid.add_cell(0, 1, 0);
    grid.add_cell(0, 0, 1);
    // Connect (0,0,0) to (0,1,0) and (0,0,1)
    grid.add_neighbor((0, 0, 0), (0, 1, 0));
    grid.add_neighbor((0, 0, 0), (0, 0, 1));

    let cell = CellKey::Square { x: 0, y: 0, z: 0 };
    let neighbors = grid.neighbors(&cell);
    assert!(neighbors.contains(&CellKey::Square { x: 0, y: 1, z: 0 }));
    assert!(neighbors.contains(&CellKey::Square { x: 0, y: 0, z: 1 }));
}

#[test]
fn test_entities_in_cell_and_zlevel() {
    // Load schemas
    let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas");
    let allowed_modes = load_allowed_modes().unwrap();
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &allowed_modes).unwrap();
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry);
    let mut grid = SquareGridMap::new();
    grid.add_cell(1, 2, 0);
    grid.add_cell(1, 2, 1);
    world.map = Some(engine_core::map::Map::new(Box::new(grid)));

    // Spawn entities and assign positions
    let eid1 = world.spawn_entity();
    let eid2 = world.spawn_entity();
    let eid3 = world.spawn_entity();

    // Place eid1 at (1,2,0), eid2 at (1,2,1), eid3 at (1,2,0)
    world
        .set_component(
            eid1,
            "Position",
            serde_json::json!({"pos": {"Square": {"x": 1, "y": 2, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(
            eid2,
            "Position",
            serde_json::json!({"pos": {"Square": {"x": 1, "y": 2, "z": 1}}}),
        )
        .unwrap();
    world
        .set_component(
            eid3,
            "Position",
            serde_json::json!({"pos": {"Square": {"x": 1, "y": 2, "z": 0}}}),
        )
        .unwrap();

    // Query all entities in (1,2,0)
    let entities_in_cell = world.entities_in_cell(&CellKey::Square { x: 1, y: 2, z: 0 });
    assert!(entities_in_cell.contains(&eid1));
    assert!(entities_in_cell.contains(&eid3));
    assert!(!entities_in_cell.contains(&eid2));

    // Query all entities in z=0
    let entities_in_z0 = world.entities_in_zlevel(0);
    assert!(entities_in_z0.contains(&eid1));
    assert!(entities_in_z0.contains(&eid3));
    assert!(!entities_in_z0.contains(&eid2));
}

#[test]
fn test_cells_in_multiple_regions() {
    // Use the shared helper to load schemas via config
    let mut world = world_helper::make_test_world();

    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    world.map = Some(engine_core::map::Map::new(Box::new(grid)));

    // Construct 'cell' value explicitly with only the "Square" key
    let mut cell_map = Map::new();
    cell_map.insert("Square".to_string(), json!({ "x": 0, "y": 0, "z": 0 }));

    // Assign cell (0,0,0) to both "room_1" and "biome_A"
    let eid = world.spawn_entity();
    let region_assignment = json!({
        "cell": Value::Object(cell_map),
        "region_id": ["room_1", "biome_A"],
    });
    world
        .set_component(eid, "RegionAssignment", region_assignment)
        .unwrap();

    // Assign cell (1,0,0) to just "room_1" with direct json! macro, as no conflict here
    let eid2 = world.spawn_entity();
    world
        .set_component(
            eid2,
            "RegionAssignment",
            json!({
                "cell": { "Square": { "x": 1, "y": 0, "z": 0 } },
                "region_id": "room_1"
            }),
        )
        .unwrap();

    // Query all cells in "room_1"
    let cells_room = world.cells_in_region("room_1");
    assert!(cells_room.contains(&json!({"Square": {"x": 0, "y": 0, "z": 0}})));
    assert!(cells_room.contains(&json!({"Square": {"x": 1, "y": 0, "z": 0}})));

    // Query all cells in "biome_A"
    let cells_biome = world.cells_in_region("biome_A");
    assert!(cells_biome.contains(&json!({"Square": {"x": 0, "y": 0, "z": 0}})));
    assert!(!cells_biome.contains(&json!({"Square": {"x": 1, "y": 0, "z": 0}})));
}

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

#[test]
fn test_set_and_get_cell_metadata() {
    let mut grid = SquareGridMap::new();
    grid.add_cell(1, 2, 0);
    let key = CellKey::Square { x: 1, y: 2, z: 0 };
    grid.set_cell_metadata(&key, json!({"biome": "Forest", "terrain": "Grass"}));
    let meta = grid.get_cell_metadata(&key).unwrap();
    assert_eq!(meta["biome"], "Forest");
    assert_eq!(meta["terrain"], "Grass");
}

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
