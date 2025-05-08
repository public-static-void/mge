//! Tests for the ECS component registry, schema handling, and migration logic.

use engine_core::ecs::Component;
use engine_core::ecs::{ComponentRegistry, Health, Position};
use semver::Version;

#[test]
fn test_component_registration() {
    let mut registry = ComponentRegistry::new();
    registry.register::<Position>().unwrap();
    assert!(registry.get_schema::<Position>().is_some());

    let json = registry.schema_to_json::<Position>().unwrap();
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
fn test_component_migration() {
    use bson::{doc, to_vec};
    use engine_core::ecs::{Component, Position};

    // Create test data
    let old_position = doc! { "x": 5.0, "y": 3.0 };
    let data = to_vec(&old_position).unwrap();

    // Perform migration
    let position = Position::migrate(Position::version(), &data).unwrap();

    assert_eq!(position.x, 5.0);
    assert_eq!(position.y, 3.0);
}

#[test]
fn test_version_migration() {
    use engine_core::ecs::{MigrationError, Position};
    use semver::Version;

    // Legacy v1 format
    #[derive(serde::Serialize)]
    struct LegacyPosition {
        x: f32,
        y: f32,
    }

    let old_pos = LegacyPosition { x: 5.0, y: 3.0 };
    let data = bson::to_vec(&old_pos).unwrap();

    // Test migration from v1.0.0
    let pos = Position::migrate(Version::parse("1.0.0").unwrap(), &data).unwrap();
    assert_eq!(pos.x, 5.0);
    assert_eq!(pos.y, 3.0);

    // Test invalid version
    let result = Position::migrate(Version::parse("3.0.0").unwrap(), &data);
    assert!(matches!(result, Err(MigrationError::UnsupportedVersion(_))));
}

#[test]
fn test_macro_generated_migration() {
    #[derive(serde::Serialize)]
    struct LegacyPosition {
        x: f32,
        y: f32,
    }

    let data = bson::to_vec(&LegacyPosition { x: 5.0, y: 3.0 }).unwrap();
    let pos = Position::migrate(Version::parse("1.0.0").unwrap(), &data).unwrap();
    assert_eq!(pos.x, 5.0);
    assert_eq!(pos.y, 3.0);
}

#[test]
fn test_external_schema_loading() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::schema::load_schemas_from_dir;

    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    assert!(
        schemas.contains_key("Health"),
        "Health schema should be loaded"
    );

    let mut registry = ComponentRegistry::default();

    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }

    // Now you can check that the registry has the schema
    assert!(registry.get_schema_by_name("Health").is_some());
}
