#[test]
fn test_get_component_mode_enforcement() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": { "mana": { "type": "number" } },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let id = world.spawn_entity();

    // In allowed mode
    world.current_mode = "colony".to_string();
    world
        .set_component(id, "MagicPower", json!({ "mana": 42 }))
        .unwrap();
    assert!(
        world.get_component(id, "MagicPower").is_some(),
        "Should be able to get component in allowed mode"
    );

    // In disallowed mode
    world.current_mode = "roguelike".to_string();
    assert!(
        world.get_component(id, "MagicPower").is_none(),
        "Should NOT be able to get component in disallowed mode"
    );
}

#[test]
fn test_remove_component_mode_enforcement() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": { "mana": { "type": "number" } },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let id = world.spawn_entity();

    world.current_mode = "colony".to_string();
    world
        .set_component(id, "MagicPower", json!({ "mana": 42 }))
        .unwrap();
    // Should succeed
    assert!(world.remove_component(id, "MagicPower").is_ok());

    // Reset for next test
    world
        .set_component(id, "MagicPower", json!({ "mana": 99 }))
        .unwrap();
    world.current_mode = "roguelike".to_string();
    // Should fail
    assert!(world.remove_component(id, "MagicPower").is_err());
}

#[test]
fn test_get_entities_with_component_mode_enforcement() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": { "mana": { "type": "number" } },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let id = world.spawn_entity();
    world.current_mode = "colony".to_string();
    world
        .set_component(id, "MagicPower", json!({ "mana": 42 }))
        .unwrap();

    // Allowed mode
    let entities = world.get_entities_with_component("MagicPower");
    assert_eq!(entities, vec![id]);

    // Disallowed mode
    world.current_mode = "roguelike".to_string();
    let entities = world.get_entities_with_component("MagicPower");
    assert!(entities.is_empty());
}
