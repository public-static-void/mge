use engine_core::ecs::registry::ComponentRegistry;
use engine_core::scripting::World;
use std::sync::Arc;
use tempfile::NamedTempFile;

#[test]
fn save_and_load_world_roundtrip() {
    // Setup registry and load schemas
    let mut registry = ComponentRegistry::new();

    // Load all JSON schemas from the directory
    let schema_dir = std::path::Path::new("../assets/schemas");
    for entry in std::fs::read_dir(schema_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let json = std::fs::read_to_string(&path).unwrap();
            registry.register_external_schema_from_json(&json).unwrap();
        }
    }
    let registry = Arc::new(registry);

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();

    // Spawn entities and set components
    let e1 = world.spawn_entity();
    world
        .set_component(
            e1,
            "Health",
            serde_json::json!({ "current": 42, "max": 100 }),
        )
        .unwrap();

    let e2 = world.spawn_entity();
    world
        .set_component(e2, "Position", serde_json::json!({ "x": 1, "y": 2 }))
        .unwrap();

    // Save world to file
    let file = NamedTempFile::new().unwrap();
    world.save_to_file(file.path()).unwrap();

    // Load world from file
    let loaded_world = World::load_from_file(file.path(), registry.clone()).unwrap();

    // Assert entities/components are identical
    assert_eq!(world.entities, loaded_world.entities);
    assert_eq!(
        world.get_component(e1, "Health"),
        loaded_world.get_component(e1, "Health")
    );
    assert_eq!(
        world.get_component(e2, "Position"),
        loaded_world.get_component(e2, "Position")
    );
}
