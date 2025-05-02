use engine_core::ecs::Component;
use engine_core::ecs::{ComponentRegistry, Health, Position};
use semver::Version;

#[test]
fn test_component_registration() {
    let mut registry = ComponentRegistry::new();
    registry.register::<Position>().unwrap();
    assert!(registry.get_schema::<Position>().is_some());

    let json = registry.schema_to_json::<Position>().unwrap();
    assert!(json.contains("X coordinate"));
    assert!(json.contains("Position"));
}

#[test]
fn test_health_component() {
    let mut registry = ComponentRegistry::new();
    registry.register::<Health>().unwrap();

    let json = registry.schema_to_json::<Health>().unwrap();

    let normalized_json = json.replace("\n", "").replace(" ", "");

    println!("Normalized JSON: {}", normalized_json);
    println!("Searching for: minimum\":0.0");
    println!("Searching for: minimum\":1.0");

    assert!(
        normalized_json.contains(r#"minimum":0.0"#),
        "Could not find minimum:0.0 in normalized JSON:\n{}",
        json
    );
    assert!(
        normalized_json.contains(r#"minimum":1.0"#),
        "Could not find minimum:1.0 in normalized JSON:\n{}",
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
        pos_x: f32,
        pos_y: f32,
    }

    let old_pos = LegacyPosition {
        pos_x: 5.0,
        pos_y: 3.0,
    };
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
