use engine_core::ecs::Component;
use engine_core::ecs::components::position::{Position, PositionComponent};
use semver::Version;

#[test]
fn test_component_migration() {
    use bson::{doc, to_vec};

    // Create test data
    let old_position = doc! { "x": 5.0, "y": 3.0 };
    let data = to_vec(&old_position).unwrap();

    // Perform migration
    let position = PositionComponent::migrate(Version::parse("1.0.0").unwrap(), &data).unwrap();

    if let Position::Square { x, y, .. } = position.pos {
        assert_eq!(x, 5);
        assert_eq!(y, 3);
        // (z as needed)
    } else {
        panic!("Expected Position::Square");
    }
}

#[test]
fn test_version_migration() {
    use engine_core::ecs::MigrationError;
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
    let pos = PositionComponent::migrate(Version::parse("1.0.0").unwrap(), &data).unwrap();
    if let Position::Square { x, y, .. } = pos.pos {
        assert_eq!(x, 5);
        assert_eq!(y, 3);
        // (z as needed)
    } else {
        panic!("Expected Position::Square");
    }

    // Test invalid version
    let result = PositionComponent::migrate(Version::parse("3.0.0").unwrap(), &data);
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
    let pos = PositionComponent::migrate(Version::parse("1.0.0").unwrap(), &data).unwrap();
    if let Position::Square { x, y, .. } = pos.pos {
        assert_eq!(x, 5);
        assert_eq!(y, 3);
    } else {
        panic!("Expected Position::Square");
    }
}
