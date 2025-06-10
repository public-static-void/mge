use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::{load_allowed_modes, load_schemas_from_dir_with_modes};
use engine_core::ecs::world::World;
use engine_core::map::{CellKey, MapTopology, SquareGridMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
