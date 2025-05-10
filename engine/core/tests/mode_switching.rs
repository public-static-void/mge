#[test]
fn test_schema_driven_mode_enforcement() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::scripting::world::World;
    use serde_json::json;
    use std::sync::Arc;

    let inventory_schema = r#"
    {
      "title": "Inventory",
      "type": "object",
      "properties": {
        "slots": { "type": "array", "items": { "type": "string" } },
        "weight": { "type": "number" }
      },
      "required": ["slots", "weight"],
      "modes": ["roguelike"]
    }
    "#;

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(inventory_schema)
        .unwrap();
    let registry = Arc::new(registry);

    let mut world = World::new(registry.clone());
    let entity = world.spawn_entity();

    world.current_mode = "colony".to_string();
    let result = world.set_component(entity, "Inventory", json!({"slots": [], "weight": 0.0}));
    assert!(
        result.is_err(),
        "Inventory should NOT be allowed in colony mode"
    );

    world.current_mode = "roguelike".to_string();
    let result = world.set_component(entity, "Inventory", json!({"slots": [], "weight": 0.0}));
    assert!(
        result.is_ok(),
        "Inventory should be allowed in roguelike mode"
    );
}
