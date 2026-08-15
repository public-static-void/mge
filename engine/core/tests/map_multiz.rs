use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::{load_allowed_modes, load_schemas_from_dir_with_modes};
use engine_core::ecs::world::World;
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::map::{CellKey, MapTopology, SquareGridMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[path = "helpers/world.rs"]
mod world_helper;
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
fn test_entities_in_zlevel_matches_hex_and_square() {
    let mut world = make_test_world();

    let hex_eid = world.spawn_entity();
    let square_eid = world.spawn_entity();
    let province_eid = world.spawn_entity();

    world
        .set_component(
            hex_eid,
            "Position",
            serde_json::json!({"pos": {"Hex": {"q": 1, "r": 2, "z": 1}}}),
        )
        .unwrap();
    world
        .set_component(
            square_eid,
            "Position",
            serde_json::json!({"pos": {"Square": {"x": 1, "y": 2, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(
            province_eid,
            "Position",
            serde_json::json!({"pos": {"Province": {"id": "A"}}}),
        )
        .unwrap();

    let on_level_one = world.entities_in_zlevel(1);
    assert!(on_level_one.contains(&hex_eid));
    assert!(!on_level_one.contains(&square_eid));
    assert!(!on_level_one.contains(&province_eid));

    let on_level_zero = world.entities_in_zlevel(0);
    assert!(on_level_zero.contains(&square_eid));
    assert!(!on_level_zero.contains(&hex_eid));
}

#[test]
fn test_move_entity_3d_moves_square_position_including_z() {
    let mut world = make_test_world();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            serde_json::json!({"pos": {"Square": {"x": 1, "y": 2, "z": 0}}}),
        )
        .unwrap();

    world.move_entity_3d(entity, 1.0, -1.0, 3.0);

    let pos = world.get_component(entity, "Position").unwrap().clone();
    assert_eq!(
        pos["pos"]["Square"],
        serde_json::json!({"x": 2, "y": 1, "z": 3})
    );
}

#[test]
fn test_move_entity_3d_moves_hex_position_including_z() {
    let mut world = make_test_world();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            serde_json::json!({"pos": {"Hex": {"q": 1, "r": 2, "z": 5}}}),
        )
        .unwrap();

    world.move_entity_3d(entity, 1.0, -2.0, -5.0);

    let pos = world.get_component(entity, "Position").unwrap().clone();
    assert_eq!(
        pos["pos"]["Hex"],
        serde_json::json!({"q": 2, "r": 0, "z": 0})
    );
}

#[test]
fn test_move_entity_3d_leaves_province_position_unchanged() {
    let mut world = make_test_world();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            serde_json::json!({"pos": {"Province": {"id": "A"}}}),
        )
        .unwrap();

    world.move_entity_3d(entity, 1.0, 1.0, 1.0);

    let pos = world.get_component(entity, "Position").unwrap().clone();
    assert_eq!(pos["pos"]["Province"], serde_json::json!({"id": "A"}));
}

#[test]
fn test_move_entity_3d_preserves_other_components() {
    let mut world = make_test_world();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            serde_json::json!({"pos": {"Square": {"x": 1, "y": 2, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(entity, "Type", serde_json::json!({"kind": "unit"}))
        .unwrap();

    world.move_entity_3d(entity, 0.0, 0.0, 4.0);

    let pos = world.get_component(entity, "Position").unwrap().clone();
    assert_eq!(pos["pos"]["Square"]["z"], 4);
    let kind = world.get_component(entity, "Type").unwrap().clone();
    assert_eq!(kind["kind"], "unit");
}

#[test]
fn test_wasm_move_entity_preserves_flat_square_z_and_extra_fields() {
    let mut world = WasmWorld::new();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            &serde_json::to_string(&serde_json::json!({
                "x": 1.0, "y": 2.0, "z": 5.0, "extra": "keep"
            }))
            .unwrap(),
        )
        .unwrap();

    world.move_entity(entity, 1.0, 0.0);

    let pos: serde_json::Value =
        serde_json::from_str(&world.get_component(entity, "Position").unwrap()).unwrap();
    assert_eq!(pos["x"], 2.0);
    assert_eq!(pos["y"], 2.0);
    assert_eq!(pos["z"], 5.0);
    assert_eq!(pos["extra"], "keep");
}

#[test]
fn test_wasm_move_entity_mutates_flat_hex_position() {
    let mut world = WasmWorld::new();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            &serde_json::to_string(&serde_json::json!({"q": 1.0, "r": 2.0, "z": 5.0})).unwrap(),
        )
        .unwrap();

    world.move_entity(entity, 1.0, -1.0);

    let pos: serde_json::Value =
        serde_json::from_str(&world.get_component(entity, "Position").unwrap()).unwrap();
    assert_eq!(pos["q"], 2.0);
    assert_eq!(pos["r"], 1.0);
    assert_eq!(pos["z"], 5.0);
}

#[test]
fn test_wasm_move_entity_creates_flat_default_with_z() {
    let mut world = WasmWorld::new();
    let entity = world.spawn_entity();

    world.move_entity(entity, 3.0, 4.0);

    let pos: serde_json::Value =
        serde_json::from_str(&world.get_component(entity, "Position").unwrap()).unwrap();
    assert_eq!(pos["x"], 3.0);
    assert_eq!(pos["y"], 4.0);
    assert_eq!(pos["z"], 0.0);
}

#[test]
fn test_wasm_move_entity_leaves_province_unchanged() {
    let mut world = WasmWorld::new();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            &serde_json::to_string(&serde_json::json!({"id": "A"})).unwrap(),
        )
        .unwrap();

    world.move_entity(entity, 1.0, 1.0);

    let pos: serde_json::Value =
        serde_json::from_str(&world.get_component(entity, "Position").unwrap()).unwrap();
    assert_eq!(pos, serde_json::json!({"id": "A"}));
}

#[test]
fn test_wasm_move_entity_3d_updates_flat_square_position() {
    let mut world = WasmWorld::new();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            &serde_json::to_string(&serde_json::json!({
                "x": 1.0, "y": 2.0, "z": 5.0, "extra": "keep"
            }))
            .unwrap(),
        )
        .unwrap();

    world.move_entity_3d(entity, 1.0, -1.0, 2.0);

    let pos: serde_json::Value =
        serde_json::from_str(&world.get_component(entity, "Position").unwrap()).unwrap();
    assert_eq!(pos["x"], 2.0);
    assert_eq!(pos["y"], 1.0);
    assert_eq!(pos["z"], 7.0);
    assert_eq!(pos["extra"], "keep");
}

#[test]
fn test_wasm_move_entity_3d_updates_flat_hex_position() {
    let mut world = WasmWorld::new();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            &serde_json::to_string(&serde_json::json!({"q": 1.0, "r": 2.0, "z": 5.0})).unwrap(),
        )
        .unwrap();

    world.move_entity_3d(entity, 1.0, 0.0, -2.0);

    let pos: serde_json::Value =
        serde_json::from_str(&world.get_component(entity, "Position").unwrap()).unwrap();
    assert_eq!(pos["q"], 2.0);
    assert_eq!(pos["r"], 2.0);
    assert_eq!(pos["z"], 3.0);
}

#[test]
fn test_wasm_move_entity_3d_leaves_province_unchanged() {
    let mut world = WasmWorld::new();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            &serde_json::to_string(&serde_json::json!({"id": "A"})).unwrap(),
        )
        .unwrap();

    world.move_entity_3d(entity, 1.0, 1.0, 1.0);

    let pos: serde_json::Value =
        serde_json::from_str(&world.get_component(entity, "Position").unwrap()).unwrap();
    assert_eq!(pos, serde_json::json!({"id": "A"}));
}

#[test]
fn test_wasm_entities_in_zlevel_matches_flat_square_and_hex() {
    let mut world = WasmWorld::new();
    let square_eid = world.spawn_entity();
    let hex_eid = world.spawn_entity();
    let province_eid = world.spawn_entity();

    world
        .set_component(
            square_eid,
            "Position",
            &serde_json::to_string(&serde_json::json!({"x": 1.0, "y": 2.0, "z": 0.0})).unwrap(),
        )
        .unwrap();
    world
        .set_component(
            hex_eid,
            "Position",
            &serde_json::to_string(&serde_json::json!({"q": 1.0, "r": 2.0, "z": 1.0})).unwrap(),
        )
        .unwrap();
    world
        .set_component(
            province_eid,
            "Position",
            &serde_json::to_string(&serde_json::json!({"id": "A"})).unwrap(),
        )
        .unwrap();

    let on_level_zero = world.entities_in_zlevel(0);
    assert!(on_level_zero.contains(&square_eid));
    assert!(!on_level_zero.contains(&hex_eid));
    assert!(!on_level_zero.contains(&province_eid));

    let on_level_one = world.entities_in_zlevel(1);
    assert!(on_level_one.contains(&hex_eid));
    assert!(!on_level_one.contains(&square_eid));
    assert!(!on_level_one.contains(&province_eid));
}
