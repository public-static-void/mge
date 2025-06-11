#[test]
fn test_get_entities_with_components() {
    use engine_core::config::GameConfig;
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    // Load config and all schemas from disk
    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schemas =
        load_schemas_from_dir_with_modes("../../engine/assets/schemas", &config.allowed_modes)
            .unwrap();
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());

    let e1 = world.spawn_entity();
    let e2 = world.spawn_entity();
    let e3 = world.spawn_entity();

    world
        .set_component(e1, "Health", json!({"current": 10, "max": 10}))
        .unwrap();
    world
        .set_component(
            e1,
            "Position",
            json!({"pos": { "Square": { "x": 1, "y": 2, "z": 0 } } }),
        )
        .unwrap();

    world
        .set_component(e2, "Health", json!({"current": 5, "max": 10}))
        .unwrap();

    world
        .set_component(
            e3,
            "Position",
            json!({"pos": { "Square": { "x": 3, "y": 4, "z": 0 } } }),
        )
        .unwrap();

    let both = world.get_entities_with_components(&["Health", "Position"]);
    assert_eq!(both, vec![e1]);
}
