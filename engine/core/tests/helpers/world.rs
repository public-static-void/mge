use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Creates a World with all schemas loaded from config.
pub fn make_test_world() -> World {
    let registry = setup_registry_from_config();
    World::new(registry)
}

/// Loads the game config and all schemas as in production.
/// Panics if config or schemas cannot be loaded.
fn setup_registry_from_config() -> Arc<Mutex<ComponentRegistry>> {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml");
    let config = GameConfig::load_from_file(&config_path)
        .unwrap_or_else(|_| panic!("Failed to load config from {config_path:?}"));
    let schema_dir = "../../engine/assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(schema_dir, &config.allowed_modes)
        .unwrap_or_else(|_| panic!("Failed to load schemas from {schema_dir:?}"));
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    Arc::new(Mutex::new(registry))
}
