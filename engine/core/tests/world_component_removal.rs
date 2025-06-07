use engine_core::ecs::ComponentSchema;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use schemars::Schema;
use std::sync::{Arc, Mutex};

#[test]
fn test_component_data_cleanup_on_unregister() {
    let mut registry = ComponentRegistry::new();
    let schema = ComponentSchema {
        name: "CleanupComponent".to_string(),
        schema: Schema::default().into(),
        modes: vec!["colony".to_string()],
    };
    registry.register_external_schema(schema);

    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    let eid = world.spawn_entity();
    world
        .set_component(eid, "CleanupComponent", serde_json::json!({"foo": 1}))
        .unwrap();
    assert!(world.get_component(eid, "CleanupComponent").is_some());

    world.unregister_component_and_cleanup("CleanupComponent");
    assert!(world.get_component(eid, "CleanupComponent").is_none());
}
