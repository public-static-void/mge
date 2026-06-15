use engine_core::ecs::registry::{ComponentRegistry, ComponentSchema};
use engine_core::ecs::registry::RegistryError;
use engine_core::ecs::components::Health;
use serde_json::json;

fn empty_registry() -> ComponentRegistry {
    ComponentRegistry::new()
}

fn health_type_name() -> &'static str {
    std::any::type_name::<Health>()
}

// --- Registration / unregistration ---

#[test]
fn test_new_is_empty_len() {
    let reg = empty_registry();
    assert!(reg.all_component_names().is_empty());
    assert!(reg.all_modes().is_empty());
    assert!(!reg.is_registered("anything"));
}

#[test]
fn test_register_component_roundtrip() {
    let mut reg = empty_registry();
    reg.register_component::<Health>().unwrap();
    let schema = reg.get_schema::<Health>();
    assert!(schema.is_some());
    assert_eq!(schema.unwrap().name, health_type_name());
}

#[test]
fn test_unregister_component() {
    let mut reg = empty_registry();
    reg.register_component::<Health>().unwrap();
    reg.unregister_component::<Health>();
    assert!(reg.get_schema::<Health>().is_none());
}

#[test]
fn test_schema_to_json_unregistered() {
    let reg = empty_registry();
    let result = reg.schema_to_json::<Health>();
    assert!(matches!(result, Err(RegistryError::UnregisteredComponent)));
}

#[test]
fn test_register_external_schema_roundtrip() {
    let mut reg = empty_registry();
    let schema = ComponentSchema {
        name: "TestComponent".into(),
        schema: json!({ "type": "object" }),
        modes: vec![],
    };
    reg.register_external_schema(schema);
    let retrieved = reg.get_schema_by_name("TestComponent");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "TestComponent");
}

#[test]
fn test_unregister_external_schema() {
    let mut reg = empty_registry();
    let schema = ComponentSchema {
        name: "TestComponent".into(),
        schema: json!({}),
        modes: vec![],
    };
    reg.register_external_schema(schema);
    reg.unregister_external_schema("TestComponent");
    assert!(reg.get_schema_by_name("TestComponent").is_none());
}

#[test]
fn test_register_external_from_json() {
    let mut reg = empty_registry();
    let json_str = r#"{"title": "FromJson", "type": "object", "modes": ["Colony"]}"#;
    reg.register_external_schema_from_json(json_str).unwrap();
    let schema = reg.get_schema_by_name("FromJson");
    assert!(schema.is_some());
    assert_eq!(schema.unwrap().name, "FromJson");
}

#[test]
fn test_register_external_from_json_missing_title() {
    let mut reg = empty_registry();
    let json_str = r#"{"type": "object"}"#;
    let result = reg.register_external_schema_from_json(json_str);
    assert!(result.is_err());
}

#[test]
fn test_register_external_from_json_empty_string() {
    let mut reg = empty_registry();
    let result = reg.register_external_schema_from_json("");
    assert!(result.is_err());
}

// --- Error paths ---

#[test]
fn test_schema_to_json_empty_registry() {
    let reg = empty_registry();
    let result = reg.schema_to_json::<Health>();
    assert!(matches!(result, Err(RegistryError::UnregisteredComponent)));
}

// --- Query operations ---

#[test]
fn test_is_registered_native_and_external() {
    let mut reg = empty_registry();
    reg.register_component::<Health>().unwrap();
    let ext = ComponentSchema {
        name: "External".into(),
        schema: json!({}),
        modes: vec![],
    };
    reg.register_external_schema(ext);

    assert!(reg.is_registered(health_type_name()));
    assert!(reg.is_registered("External"));
    assert!(!reg.is_registered("NonExistent"));
}

#[test]
fn test_all_component_names() {
    let mut reg = empty_registry();
    reg.register_component::<Health>().unwrap();
    let ext = ComponentSchema {
        name: "ExternalComp".into(),
        schema: json!({}),
        modes: vec![],
    };
    reg.register_external_schema(ext);

    let names = reg.all_component_names();
    assert!(names.contains(&health_type_name().to_string()));
    assert!(names.contains(&"ExternalComp".to_string()));
    assert_eq!(names.len(), 2);
}

#[test]
fn test_all_modes_deduplicated() {
    let mut reg = empty_registry();
    let schema_a = ComponentSchema {
        name: "A".into(),
        schema: json!({}),
        modes: vec!["Colony".into(), "Roguelike".into()],
    };
    let schema_b = ComponentSchema {
        name: "B".into(),
        schema: json!({}),
        modes: vec!["Colony".into()],
    };
    reg.register_external_schema(schema_a);
    reg.register_external_schema(schema_b);

    let modes = reg.all_modes();
    assert!(modes.contains("Colony"));
    assert!(modes.contains("Roguelike"));
    assert_eq!(modes.len(), 2);
}

#[test]
fn test_components_for_mode() {
    let mut reg = empty_registry();
    let schema = ComponentSchema {
        name: "ModeComp".into(),
        schema: json!({}),
        modes: vec!["Dungeon".into()],
    };
    reg.register_external_schema(schema);

    let matched = reg.components_for_mode("Dungeon");
    assert_eq!(matched, vec!["ModeComp"]);

    let no_match = reg.components_for_mode("Nonexistent");
    assert!(no_match.is_empty());
}

#[test]
fn test_components_for_mode_no_match() {
    let reg = empty_registry();
    let result = reg.components_for_mode("MissingMode");
    assert!(result.is_empty());
}

// --- Update / hot-reload ---

#[test]
fn test_update_external_schema_replace() {
    let mut reg = empty_registry();
    let original = ComponentSchema {
        name: "Updatable".into(),
        schema: json!({ "version": 1 }),
        modes: vec![],
    };
    reg.register_external_schema(original);

    let updated = ComponentSchema {
        name: "Updatable".into(),
        schema: json!({ "version": 2 }),
        modes: vec!["NewMode".into()],
    };
    reg.update_external_schema(updated).unwrap();

    let retrieved = reg.get_schema_by_name("Updatable").unwrap();
    assert_eq!(retrieved.schema, json!({ "version": 2 }));
    assert_eq!(retrieved.modes, vec!["NewMode"]);
}

#[test]
fn test_update_external_schema_insert() {
    let mut reg = empty_registry();
    let schema = ComponentSchema {
        name: "NewInsert".into(),
        schema: json!({ "key": "value" }),
        modes: vec![],
    };
    reg.update_external_schema(schema).unwrap();
    assert!(reg.get_schema_by_name("NewInsert").is_some());
}

#[test]
fn test_update_external_schema_with_migration() {
    let mut reg = empty_registry();
    let schema = ComponentSchema {
        name: "Migrated".into(),
        schema: json!({ "version": 2 }),
        modes: vec![],
    };
    let mut data: std::collections::HashMap<u32, serde_json::Value> =
        [(1u32, json!({ "val": 10 })), (2u32, json!({ "val": 20 }))]
            .into_iter()
            .collect();

    reg.update_external_schema_with_migration(schema, &mut data, |v| {
        let mut map = v.as_object().unwrap().clone();
        map.insert("migrated".into(), json!(true));
        serde_json::Value::Object(map)
    })
    .unwrap();

    assert_eq!(data.get(&1u32).unwrap()["migrated"], json!(true));
    assert_eq!(data.get(&2u32).unwrap()["migrated"], json!(true));
    assert!(reg.get_schema_by_name("Migrated").is_some());
}

// --- Edge cases ---

#[test]
fn test_unregister_component_never_registered() {
    let mut reg = empty_registry();
    reg.unregister_component::<Health>();
    // Should not panic
}

#[test]
fn test_unregister_external_schema_never_registered() {
    let mut reg = empty_registry();
    reg.unregister_external_schema("NeverRegistered");
    // Should not panic
}

#[test]
fn test_register_external_schema_overwrite() {
    let mut reg = empty_registry();
    let first = ComponentSchema {
        name: "Overwrite".into(),
        schema: json!({ "data": "first" }),
        modes: vec![],
    };
    reg.register_external_schema(first);
    let second = ComponentSchema {
        name: "Overwrite".into(),
        schema: json!({ "data": "second" }),
        modes: vec![],
    };
    reg.register_external_schema(second);

    let retrieved = reg.get_schema_by_name("Overwrite").unwrap();
    assert_eq!(retrieved.schema, json!({ "data": "second" }));
}

#[test]
fn test_get_schema_by_name_empty_registry() {
    let reg = empty_registry();
    assert!(reg.get_schema_by_name("Anything").is_none());
}
