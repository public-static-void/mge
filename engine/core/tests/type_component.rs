use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::scripting::World;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_set_and_get_type_component() {
    // Load schemas and set up registry
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        for (_name, schema) in schemas {
            reg.register_external_schema(schema);
        }
    }

    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    let id = world.spawn_entity();
    let type_value = json!({ "kind": "player" });
    world.set_component(id, "Type", type_value.clone()).unwrap();

    let stored = world.get_component(id, "Type").unwrap();
    assert_eq!(stored, &type_value);
}
