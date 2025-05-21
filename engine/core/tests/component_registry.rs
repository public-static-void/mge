use engine_core::ecs::components::position::PositionComponent;
use engine_core::ecs::{ComponentRegistry, Health};

#[test]
fn test_component_registration() {
    let mut registry = ComponentRegistry::new();
    registry.register::<PositionComponent>().unwrap();
    assert!(registry.get_schema::<PositionComponent>().is_some());

    let json = registry.schema_to_json::<PositionComponent>().unwrap();
    println!("Position schema: {}", json);
    assert!(
        json.contains("\"x\""),
        "Schema does not contain field 'x':\n{}",
        json
    );
    assert!(
        json.contains("Position"),
        "Schema does not mention 'Position':\n{}",
        json
    );
}

#[test]
fn test_health_component() {
    let mut registry = ComponentRegistry::new();
    registry.register::<Health>().unwrap();
    assert!(registry.get_schema::<Health>().is_some());

    let json = registry.schema_to_json::<Health>().unwrap();
    println!("Health schema: {}", json);
    assert!(
        json.contains("\"current\""),
        "Schema does not contain field 'current':\n{}",
        json
    );
    assert!(
        json.contains("\"max\""),
        "Schema does not contain field 'max':\n{}",
        json
    );
}

#[test]
fn test_unregistered_component() {
    use engine_core::ecs::RegistryError;

    let registry = ComponentRegistry::new();
    let result = registry.schema_to_json::<Health>();

    match result {
        Ok(_) => panic!("Expected an error, but got Ok"),
        Err(e) => match e {
            RegistryError::UnregisteredComponent => (),
            _ => panic!("Expected UnregisteredComponent error, got {:?}", e),
        },
    }
}

#[test]
fn test_external_schema_loading() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::schema::load_schemas_from_dir;
    use std::sync::{Arc, Mutex};

    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    assert!(
        schemas.contains_key("Health"),
        "Health schema should be loaded"
    );

    let registry = Arc::new(Mutex::new(ComponentRegistry::default()));

    for (_name, schema) in schemas {
        registry.lock().unwrap().register_external_schema(schema);
    }

    // Now you can check that the registry has the schema
    let guard = registry.lock().unwrap();
    assert!(guard.get_schema_by_name("Health").is_some());
}

#[test]
fn test_schema_driven_mode_enforcement() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    // Fabricate a schema for "Roguelike::Inventory" only allowed in "roguelike" mode
    let roguelike_inventory_schema = r#"
    {
      "title": "Roguelike::Inventory",
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
        .register_external_schema_from_json(roguelike_inventory_schema)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let entity = world.spawn_entity();

    // Not allowed in "colony"
    world.current_mode = "colony".to_string();
    let result = world.set_component(
        entity,
        "Roguelike::Inventory",
        json!({"slots": [], "weight": 0.0}),
    );
    assert!(
        result.is_err(),
        "Roguelike::Inventory should NOT be allowed in colony mode"
    );

    // Allowed in "roguelike"
    world.current_mode = "roguelike".to_string();
    let result = world.set_component(
        entity,
        "Roguelike::Inventory",
        json!({"slots": [], "weight": 0.0}),
    );
    assert!(
        result.is_ok(),
        "Roguelike::Inventory should be allowed in roguelike mode"
    );
}

#[test]
fn test_register_external_schema_from_json() {
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();

    // Example schema JSON string
    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": {
            "mana": { "type": "number", "minimum": 0 }
        },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;

    let result = registry.register_external_schema_from_json(schema_json);
    assert!(
        result.is_ok(),
        "Schema registration failed: {:?}",
        result.err()
    );

    let registry = Arc::new(Mutex::new(registry));

    // FIX: Avoid E0716 by binding the lock guard
    let guard = registry.lock().unwrap();
    let schema = guard.get_schema_by_name("MagicPower");
    assert!(
        schema.is_some(),
        "Schema 'MagicPower' not found in registry"
    );

    // Check modes are correctly set
    let modes = &schema.unwrap().modes;
    assert!(
        modes.contains(&"colony".to_string()),
        "Mode 'colony' not set"
    );
}

#[test]
fn test_mode_enforcement_for_runtime_registered_schema() {
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": { "mana": { "type": "number" } },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let id = world.spawn_entity();

    // Allowed in "colony"
    world.current_mode = "colony".to_string();
    assert!(
        world
            .set_component(id, "MagicPower", json!({ "mana": 42 }))
            .is_ok(),
        "Should be allowed in colony mode"
    );

    // Not allowed in "roguelike"
    world.current_mode = "roguelike".to_string();
    let result = world.set_component(id, "MagicPower", json!({ "mana": 99 }));
    assert!(result.is_err(), "Should not be allowed in roguelike mode");
}

#[test]
fn test_set_component_validation() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let schema_json = r#"
    {
        "title": "TestComponent",
        "type": "object",
        "properties": {
            "value": { "type": "integer", "minimum": 0, "maximum": 10 }
        },
        "required": ["value"],
        "modes": ["colony"]
    }
    "#;
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let entity = world.spawn_entity();

    world.current_mode = "colony".to_string();

    // Valid data
    assert!(
        world
            .set_component(entity, "TestComponent", json!({ "value": 5 }))
            .is_ok()
    );

    // Invalid data (value too high)
    assert!(
        world
            .set_component(entity, "TestComponent", json!({ "value": 20 }))
            .is_err()
    );

    // Missing required field
    assert!(
        world
            .set_component(entity, "TestComponent", json!({}))
            .is_err()
    );
}
