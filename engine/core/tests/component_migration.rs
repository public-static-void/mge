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

    assert!(matches!(position.pos, Position::Square { x: 5, y: 3, .. }));
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
    assert!(matches!(pos.pos, Position::Square { x: 5, y: 3, .. }));

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
    assert!(matches!(pos.pos, Position::Square { x: 5, y: 3, .. }));
}
