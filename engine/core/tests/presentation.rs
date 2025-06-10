use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::presentation::PresentationSystem;
use engine_core::presentation::renderer::{RenderColor, TestRenderer};
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[test]
fn test_presentation_system_renders_entities() {
    // Load config and all schemas from the assets directory
    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas");
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");

    // Register all schemas
    let mut registry = ComponentRegistry::new();
    for schema in schemas.values() {
        registry.register_external_schema(schema.clone());
    }
    println!("Loaded schemas: {:?}", schemas.keys().collect::<Vec<_>>());

    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    // Spawn an entity using the existing public method
    let entity = world.spawn_entity();

    // Set PositionComponent as JSON (Square)
    world
        .set_component(
            entity,
            "Position",
            json!({
                "pos": { "Square": { "x": 1, "y": 2, "z": 0 } }
            }),
        )
        .unwrap();

    // Set Renderable as JSON
    world
        .set_component(
            entity,
            "Renderable",
            json!({
                "glyph": "@",
                "color": [255, 255, 255]
            }),
        )
        .unwrap();

    let renderer = TestRenderer::new();
    let mut system = PresentationSystem::new(renderer);

    system.render_world(&world);

    // Check draw call
    assert_eq!(system.renderer.draws.len(), 1);
    assert_eq!(system.renderer.draws[0].glyph, '@');
    assert_eq!(system.renderer.draws[0].pos, (1, 2));
    assert_eq!(system.renderer.draws[0].color, RenderColor(255, 255, 255));
}
