#[path = "helpers/registry.rs"]
mod registry;

#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::config::GameConfig;
use engine_core::ecs::ComponentSchema;
use engine_core::ecs::Health;
use engine_core::ecs::World;
use engine_core::ecs::components::position::PositionComponent;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::{
    ComponentSchema as CS, load_allowed_modes, load_schemas_from_dir_with_modes,
    load_schemas_recursively, save_schema_to_file,
};
use schemars::Schema;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

// === schema tests ===

#[test]
fn test_load_schemas_with_validation() {
    let dir = tempdir().unwrap();
    let valid_schema = r#"{
        "title": "ValidComponent",
        "modes": ["colony"],
        "type": "object",
        "properties": { "foo": { "type": "integer" } }
    }"#;
    fs::write(dir.path().join("valid.json"), valid_schema).unwrap();

    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schemas = load_schemas_from_dir_with_modes(dir.path(), &config.allowed_modes).unwrap();
    assert!(schemas.contains_key("ValidComponent"));

    let invalid_schema = r#"{
        "type": "object",
        "properties": { "foo": { "type": "integer" } }
    }"#;
    fs::write(dir.path().join("invalid.json"), invalid_schema).unwrap();

    let result = load_schemas_from_dir_with_modes(dir.path(), &config.allowed_modes);
    assert!(result.is_err());
}

#[test]
fn test_save_and_load_schema_roundtrip() {
    let dir = tempdir().unwrap();
    let schema = CS {
        name: "RoundTrip".to_string(),
        schema: serde_json::json!({
            "title": "RoundTrip",
            "modes": ["colony"],
            "type": "object"
        }),
        modes: vec!["colony".to_string()],
    };
    let path = dir.path().join("roundtrip.json");
    save_schema_to_file(&schema, &path).unwrap();

    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let loaded = load_schemas_from_dir_with_modes(dir.path(), &config.allowed_modes).unwrap();
    assert!(loaded.contains_key("RoundTrip"));
}

#[test]
fn test_load_schemas_recursively() {
    let dir = tempdir().unwrap();
    let subdir = dir.path().join("nested");
    fs::create_dir(&subdir).unwrap();

    let schema1 = r#"{
        "title": "RootComponent",
        "modes": ["colony"],
        "type": "object"
    }"#;
    let schema2 = r#"{
        "title": "NestedComponent",
        "modes": ["colony"],
        "type": "object"
    }"#;
    fs::write(dir.path().join("root.json"), schema1).unwrap();
    fs::write(subdir.join("nested.json"), schema2).unwrap();

    let allowed_modes = load_allowed_modes().expect("Failed to load allowed_modes");
    let schemas = load_schemas_recursively(dir.path(), true, &allowed_modes).unwrap();
    assert!(schemas.contains_key("RootComponent"));
    assert!(schemas.contains_key("NestedComponent"));
}

// === component_registry_registration tests ===

#[test]
fn test_component_registration() {
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

    let registry = ComponentRegistry::new();
    let result = registry.schema_to_json::<Health>();

    assert!(
        matches!(result, Err(RegistryError::UnregisteredComponent)),
        "Expected UnregisteredComponent error"
    );
}

#[test]
fn test_external_schema_loading() {
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

    let guard = registry.lock().unwrap();
    assert!(guard.get_schema_by_name("Health").is_some());
}

#[test]
fn test_register_external_schema_from_real_file() {
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

    assert!(
        world
            .set_component(entity, "TestComponent", json!({ "value": 5 }))
            .is_ok()
    );
    assert!(
        world
            .set_component(entity, "TestComponent", json!({ "value": 20 }))
            .is_err()
    );
    assert!(
        world
            .set_component(entity, "TestComponent", json!({}))
            .is_err()
    );
}

#[test]
fn test_register_and_unregister_external_schema() {
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
    let mut registry = ComponentRegistry::new();

    assert!(registry.register_component::<PositionComponent>().is_ok());
    assert!(registry.get_schema::<PositionComponent>().is_some());

    registry.unregister_component::<PositionComponent>();
    assert!(registry.get_schema::<PositionComponent>().is_none());
}

#[test]
fn test_components_for_mode() {
    let mut registry = ComponentRegistry::new();

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
    let mut registry = ComponentRegistry::new();

    registry.register_external_schema(ComponentSchema {
        name: "Foo".to_string(),
        schema: Schema::default().into(),
        modes: vec!["test".to_string()],
    });

    assert!(registry.is_registered("Foo"));
    assert!(!registry.is_registered("Bar"));
}

#[test]
fn test_is_registered_rust_native() {
    let mut registry = ComponentRegistry::new();
    assert!(!registry.is_registered(std::any::type_name::<PositionComponent>()));
    registry.register_component::<PositionComponent>().unwrap();
    assert!(registry.is_registered(std::any::type_name::<PositionComponent>()));
}

#[test]
fn test_update_external_schema_and_migrate() {
    let mut registry = ComponentRegistry::new();

    let schema_v1 = Schema::default();
    let component_v1 = ComponentSchema {
        name: "HotReloadComponent".to_string(),
        schema: schema_v1.clone().into(),
        modes: vec!["colony".to_string()],
    };
    registry.register_external_schema(component_v1);

    let schema_v2 = Schema::default();
    let component_v2 = ComponentSchema {
        name: "HotReloadComponent".to_string(),
        schema: schema_v2.clone().into(),
        modes: vec!["colony".to_string()],
    };

    registry.update_external_schema(component_v2).unwrap();

    let updated = registry.get_schema_by_name("HotReloadComponent").unwrap();
    assert_eq!(updated.schema, schema_v2);
    assert_eq!(updated.modes, vec!["colony".to_string()]);
}

#[test]
fn test_update_external_schema_with_data_migration() {
    let mut registry = ComponentRegistry::new();

    let schema_v1 = ComponentSchema {
        name: "MigratingComponent".to_string(),
        schema: Schema::default().into(),
        modes: vec!["colony".to_string()],
    };
    registry.register_external_schema(schema_v1);

    let mut component_data = HashMap::new();
    component_data.insert(1u32, json!({ "foo": 1 }));

    let migration = |old: &serde_json::Value| {
        let mut new = old.clone();
        if let Some(val) = new.get("foo").cloned() {
            new.as_object_mut().unwrap().remove("foo");
            new.as_object_mut().unwrap().insert("bar".to_string(), val);
        }
        new
    };

    let schema_v2 = ComponentSchema {
        name: "MigratingComponent".to_string(),
        schema: Schema::default().into(),
        modes: vec!["colony".to_string()],
    };
    registry
        .update_external_schema_with_migration(schema_v2, &mut component_data, migration)
        .unwrap();

    let migrated = component_data.get(&1u32).unwrap();
    assert!(migrated.get("foo").is_none());
    assert_eq!(migrated.get("bar").unwrap(), &json!(1));

    let updated = registry.get_schema_by_name("MigratingComponent").unwrap();
    assert_eq!(updated.modes, vec!["colony".to_string()]);
}

// === component_change_events tests ===

#[test]
fn test_component_change_events() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let changes: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));

    world.get_or_create_event_bus::<serde_json::Value>("component_changed");

    let changes_sub = changes.clone();
    let sub_id = world
        .subscribe::<serde_json::Value, _>("component_changed", move |event| {
            changes_sub.lock().unwrap().push(event.clone());
        })
        .unwrap();

    world
        .set_component(eid, "Health", json!({"current": 100, "max": 100}))
        .unwrap();
    world.update_event_buses::<serde_json::Value>();

    let changes_lock = changes.lock().unwrap();
    assert!(
        !changes_lock.is_empty(),
        "No component change event received"
    );
    let event = &changes_lock[0];
    assert_eq!(event["entity"], eid);
    assert_eq!(event["component"], "Health");
    assert_eq!(event["action"], "set");
    assert_eq!(event["new"]["current"], 100);
    assert_eq!(event["new"]["max"], 100);
    drop(changes_lock);

    world.remove_component(eid, "Health").unwrap();
    world.update_event_buses::<serde_json::Value>();
    let changes_lock = changes.lock().unwrap();
    assert!(changes_lock.iter().any(|e| e["action"] == "removed"));
    drop(changes_lock);

    world.unsubscribe::<serde_json::Value>("component_changed", sub_id);
    world
        .set_component(eid, "Health", json!({"current": 50, "max": 100}))
        .unwrap();
    world.update_event_buses::<serde_json::Value>();
    let changes_lock = changes.lock().unwrap();
    let set_count = changes_lock.iter().filter(|e| e["action"] == "set").count();
    assert_eq!(
        set_count, 1,
        "Should not receive new events after unsubscribe"
    );
}

// === type_component tests ===

#[test]
fn test_set_and_get_type_component() {
    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");

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
