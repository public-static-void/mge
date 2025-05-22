use engine_core::ecs::ComponentSchema;
use engine_core::ecs::registry::ComponentRegistry;
use schemars::schema::RootSchema;
use serde_json::json;

#[test]
fn test_update_external_schema_with_data_migration() {
    let mut registry = ComponentRegistry::new();

    // Register initial schema
    let schema_v1 = ComponentSchema {
        name: "MigratingComponent".to_string(),
        schema: RootSchema::default(),
        modes: vec!["colony".to_string()],
    };
    registry.register_external_schema(schema_v1);

    // Simulate world/component storage for migration
    let mut component_data = std::collections::HashMap::new();
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
        schema: RootSchema::default(),
        modes: vec!["colony".to_string()],
    };
    registry
        .update_external_schema_with_migration(schema_v2, &mut component_data, migration)
        .unwrap();

    // Data should be migrated
    let migrated = component_data.get(&1u32).unwrap();
    assert!(migrated.get("foo").is_none());
    assert_eq!(migrated.get("bar").unwrap(), &json!(1));
}
