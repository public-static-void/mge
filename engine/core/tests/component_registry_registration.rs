#[path = "helpers/registry.rs"]
mod registry;

use engine_core::ecs::ComponentSchema;
use engine_core::ecs::Health;
use engine_core::ecs::components::position::PositionComponent;
use engine_core::ecs::registry::ComponentRegistry;
use schemars::Schema;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_component_registration() {
    // Test registration of a Rust-native component and schema export
    let mut registry = ComponentRegistry::new();
    registry.register_component::<PositionComponent>().unwrap();
    assert!(registry.get_schema::<PositionComponent>().is_some());

    let json = registry.schema_to_json::<PositionComponent>().unwrap();
    assert!(
        json.contains("\"x\""),
        "Schema does not contain field 'x':\n{json}"
    );
    assert!(
        json.contains("Position"),
        "Schema does not mention 'Position':\n{json}"
    );
}

#[test]
fn test_health_component() {
    // Test registration of the Health component and schema export
    let mut registry = ComponentRegistry::new();
    registry.register_component::<Health>().unwrap();
    assert!(registry.get_schema::<Health>().is_some());

    let json = registry.schema_to_json::<Health>().unwrap();
    assert!(
        json.contains("\"current\""),
        "Schema does not contain field 'current':\n{json}"
    );
    assert!(
        json.contains("\"max\""),
        "Schema does not contain field 'max':\n{json}"
    );
}

#[test]
fn test_unregistered_component() {
    use engine_core::ecs::RegistryError;

    // Test error for unregistered component
    let registry = ComponentRegistry::new();
    let result = registry.schema_to_json::<Health>();

    assert!(
        matches!(result, Err(RegistryError::UnregisteredComponent)),
        "Expected UnregisteredComponent error"
    );
}

#[test]
fn test_external_schema_loading() {
    use engine_core::config::GameConfig;
    use engine_core::ecs::registry::ComponentRegistry;
    use std::sync::{Arc, Mutex};

    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schema_dir = "../../engine/assets/schemas";
    let schemas = engine_core::ecs::schema::load_schemas_from_dir_with_modes(
        schema_dir,
        &config.allowed_modes,
    )
    .expect("Failed to load schemas");
    assert!(
        schemas.contains_key("Health"),
        "Health schema should be loaded"
    );

    let registry = Arc::new(Mutex::new(ComponentRegistry::default()));
    for (_name, schema) in schemas {
        registry.lock().unwrap().register_external_schema(schema);
    }

    // Now check that the registry has the schema
    let guard = registry.lock().unwrap();
    assert!(guard.get_schema_by_name("Health").is_some());
}

#[test]
fn test_register_external_schema_from_real_file() {
    use std::sync::{Arc, Mutex};

    // Test registering a schema from a real file
    let mut registry = ComponentRegistry::new();
    let schema_json = registry::load_schema_from_assets("health");
    registry
        .register_external_schema_from_json(&schema_json)
        .unwrap();

    let registry = Arc::new(Mutex::new(registry));
    let guard = registry.lock().unwrap();
    let schema = guard.get_schema_by_name("Health");
    assert!(schema.is_some(), "Schema 'Health' not found in registry");
    assert!(
        schema.unwrap().modes.contains(&"colony".to_string()),
        "Health schema should be allowed in colony mode"
    );
}

#[test]
fn test_schema_driven_mode_enforcement() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    // Test mode enforcement for a custom schema only allowed in "roguelike" mode
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
fn test_set_component_validation() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    // Test schema validation for a custom schema
    let custom_schema = r#"
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

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(custom_schema)
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

#[test]
fn test_register_and_unregister_external_schema() {
    // Test registering and unregistering a custom schema
    let mut registry = ComponentRegistry::new();
    let schema = ComponentSchema {
        name: "TestComponent".to_string(),
        schema: Schema::default().into(),
        modes: vec!["test".to_string()],
    };
    registry.register_external_schema(schema.clone());
    assert!(registry.get_schema_by_name("TestComponent").is_some());

    registry.unregister_external_schema("TestComponent");
    assert!(registry.get_schema_by_name("TestComponent").is_none());
}

#[test]
fn test_register_and_unregister_rust_native_component() {
    use engine_core::ecs::components::position::PositionComponent;
    use engine_core::ecs::registry::ComponentRegistry;

    // Test registering and unregistering a Rust-native component
    let mut registry = ComponentRegistry::new();

    // Register component
    assert!(registry.register_component::<PositionComponent>().is_ok());
    assert!(registry.get_schema::<PositionComponent>().is_some());

    // Unregister component
    registry.unregister_component::<PositionComponent>();
    assert!(registry.get_schema::<PositionComponent>().is_none());
}

#[test]
fn test_components_for_mode() {
    use engine_core::ecs::ComponentSchema;
    use engine_core::ecs::registry::ComponentRegistry;
    use schemars::Schema;

    // Test querying components for a specific mode
    let mut registry = ComponentRegistry::new();

    // Register two custom components for different modes
    registry.register_external_schema(ComponentSchema {
        name: "A".to_string(),
        schema: Schema::default().into(),
        modes: vec!["foo".to_string()],
    });
    registry.register_external_schema(ComponentSchema {
        name: "B".to_string(),
        schema: Schema::default().into(),
        modes: vec!["bar".to_string()],
    });

    let foo_comps = registry.components_for_mode("foo");
    let bar_comps = registry.components_for_mode("bar");
    assert!(foo_comps.contains(&"A".to_string()));
    assert!(!foo_comps.contains(&"B".to_string()));
    assert!(bar_comps.contains(&"B".to_string()));
    assert!(!bar_comps.contains(&"A".to_string()));
}

#[test]
fn test_is_registered() {
    use engine_core::ecs::ComponentSchema;
    use engine_core::ecs::registry::ComponentRegistry;
    use schemars::Schema;

    // Test checking if a custom component is registered
    let mut registry = ComponentRegistry::new();

    // Register a custom component
    registry.register_external_schema(ComponentSchema {
        name: "Foo".to_string(),
        schema: Schema::default().into(),
        modes: vec!["test".to_string()],
    });

    // Should be registered
    assert!(registry.is_registered("Foo"));
    // Should not be registered
    assert!(!registry.is_registered("Bar"));
}

#[test]
fn test_is_registered_rust_native() {
    use engine_core::ecs::components::position::PositionComponent;
    use engine_core::ecs::registry::ComponentRegistry;

    // Test checking if a Rust-native component is registered
    let mut registry = ComponentRegistry::new();
    assert!(!registry.is_registered(std::any::type_name::<PositionComponent>()));
    registry.register_component::<PositionComponent>().unwrap();
    assert!(registry.is_registered(std::any::type_name::<PositionComponent>()));
}

#[test]
fn test_update_external_schema_and_migrate() {
    let mut registry = ComponentRegistry::new();

    // Register initial schema (version 1.0.0)
    let schema_v1 = Schema::default();
    let component_v1 = ComponentSchema {
        name: "HotReloadComponent".to_string(),
        schema: schema_v1.clone().into(),
        modes: vec!["colony".to_string()],
    };
    registry.register_external_schema(component_v1);

    // Update schema (version 2.0.0)
    let schema_v2 = Schema::default();
    let component_v2 = ComponentSchema {
        name: "HotReloadComponent".to_string(),
        schema: schema_v2.clone().into(),
        modes: vec!["colony".to_string()],
    };

    // Should replace the schema
    registry.update_external_schema(component_v2).unwrap();

    let updated = registry.get_schema_by_name("HotReloadComponent").unwrap();
    assert_eq!(updated.schema, schema_v2);
    assert_eq!(updated.modes, vec!["colony".to_string()]);
}

#[test]
fn test_update_external_schema_with_data_migration() {
    let mut registry = ComponentRegistry::new();

    // Register initial schema
    let schema_v1 = ComponentSchema {
        name: "MigratingComponent".to_string(),
        schema: Schema::default().into(),
        modes: vec!["colony".to_string()],
    };
    registry.register_external_schema(schema_v1);

    // Simulate world/component storage for migration
    let mut component_data = HashMap::new();
    component_data.insert(1u32, json!({ "foo": 1 }));

    // Define migration: rename field "foo" to "bar"
    let migration = |old: &serde_json::Value| {
        let mut new = old.clone();
        if let Some(val) = new.get("foo").cloned() {
            new.as_object_mut().unwrap().remove("foo");
            new.as_object_mut().unwrap().insert("bar".to_string(), val);
        }
        new
    };

    // Update schema with migration
    let schema_v2 = ComponentSchema {
        name: "MigratingComponent".to_string(),
        schema: Schema::default().into(),
        modes: vec!["colony".to_string()],
    };
    registry
        .update_external_schema_with_migration(schema_v2, &mut component_data, migration)
        .unwrap();

    // Data should be migrated
    let migrated = component_data.get(&1u32).unwrap();
    assert!(migrated.get("foo").is_none());
    assert_eq!(migrated.get("bar").unwrap(), &json!(1));

    let updated = registry.get_schema_by_name("MigratingComponent").unwrap();
    assert_eq!(updated.modes, vec!["colony".to_string()]);
}
