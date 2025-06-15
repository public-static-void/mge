use engine_core::ecs::ComponentSchema;
use engine_core::ecs::registry::ComponentRegistry;
use schemars::Schema;

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

    // The registry should now have the updated schema
    let updated = registry.get_schema_by_name("HotReloadComponent").unwrap();
    assert_eq!(updated.schema, schema_v2);
    assert_eq!(updated.modes, vec!["colony".to_string()]);
}
