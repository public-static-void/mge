#[test]
fn test_ffi_spawn_entity_and_set_component() {
    // Setup: Create a new registry and register Position schema
    let mut registry = engine_core::ecs::registry::ComponentRegistry::new();

    let schema_json = r#"
    {
        "title": "Position",
        "type": "object",
        "properties": {
            "x": { "type": "number" },
            "y": { "type": "number" }
        },
        "required": ["x", "y"],
        "modes": ["colony", "roguelike"]
    }
    "#;
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();

    let registry = std::sync::Arc::new(registry);
    let mut world = engine_core::scripting::World::new(registry);

    // Call spawn_entity directly
    let entity_id = world.spawn_entity();
    assert!(entity_id > 0);

    // Prepare JSON component data
    let json_value = serde_json::json!({ "x": 10.0, "y": 20.0 });

    // Set component
    let result = world.set_component(entity_id, "Position", json_value);
    assert!(result.is_ok());

    // Verify component was set
    let component = world.get_component(entity_id, "Position");
    assert!(component.is_some());
}
