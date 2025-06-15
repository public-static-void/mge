use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::world::World;
use engine_core::systems::job::effect_processor_registry::EffectProcessorRegistry;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Sets up a test world and effect registry with custom schemas for effect testing.
pub fn setup_world_and_registry() -> (World, Arc<Mutex<EffectProcessorRegistry>>) {
    let mut registry = ComponentRegistry::default();
    registry.register_external_schema(ComponentSchema {
        name: "Marked".to_string(),
        schema: json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    registry.register_external_schema(ComponentSchema {
        name: "Scripted".to_string(),
        schema: json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    let world = World::new(Arc::new(Mutex::new(registry)));
    let effect_registry = Arc::new(Mutex::new(EffectProcessorRegistry::default()));
    (world, effect_registry)
}
