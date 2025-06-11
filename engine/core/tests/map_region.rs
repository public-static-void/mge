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
fn test_entities_in_region() {
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
            serde_json::json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(
            eid2,
            "Position",
            serde_json::json!({"pos": {"Square": {"x": 1, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(
            eid3,
            "Position",
            serde_json::json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();

    // Assign regions
    world
        .set_component(
            eid1,
            "Region",
            serde_json::json!({"id": "room_1", "kind": "room"}),
        )
        .unwrap();
    world
        .set_component(
            eid2,
            "Region",
            serde_json::json!({"id": "room_2", "kind": "room"}),
        )
        .unwrap();
    world
        .set_component(
            eid3,
            "Region",
            serde_json::json!({"id": "room_1", "kind": "room"}),
        )
        .unwrap();

    // Query all entities in region "room_1"
    let entities = world.entities_in_region("room_1");
    assert!(entities.contains(&eid1));
    assert!(entities.contains(&eid3));
    assert!(!entities.contains(&eid2));
}
