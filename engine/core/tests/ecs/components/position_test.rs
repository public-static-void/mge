use engine_core::ecs::Component;
use engine_core::ecs::components::position::{PositionComponent, Position, LegacyPosition};
use engine_core::ecs::error::MigrationError;
use semver::Version;

/// Test helper: re-implements LegacyPosition with Serialize for BSON test data.
/// The production LegacyPosition only derives Deserialize (used by the migration
/// function), but we need Serialize to produce BSON test input.
#[derive(serde::Serialize)]
struct TestLegacyPosition {
    x: f32,
    y: f32,
}

#[test]
fn test_version() {
    assert_eq!(
        PositionComponent::version(),
        Version::parse("3.0.0").unwrap()
    );
}

#[test]
fn test_migrate_from_v1() {
    let legacy = TestLegacyPosition { x: 1.0, y: 2.0 };
    let data = bson::to_vec(&legacy).unwrap();
    let result = PositionComponent::migrate(Version::parse("1.0.0").unwrap(), &data).unwrap();
    assert_eq!(result.pos, Position::Square { x: 1, y: 2, z: 0 });
}

#[test]
fn test_migrate_unsupported_version() {
    let legacy = TestLegacyPosition { x: 0.0, y: 0.0 };
    let data = bson::to_vec(&legacy).unwrap();
    let result = PositionComponent::migrate(Version::parse("2.0.0").unwrap(), &data);
    assert!(matches!(
        result,
        Err(MigrationError::UnsupportedVersion(v)) if v == Version::parse("2.0.0").unwrap()
    ));
}

#[test]
fn test_migrate_empty_data() {
    let result = PositionComponent::migrate(Version::parse("1.0.0").unwrap(), &[]);
    assert!(matches!(
        result,
        Err(MigrationError::DeserializationError(_))
    ));
}

#[test]
fn test_serde_roundtrip() {
    let original = PositionComponent {
        pos: Position::Square { x: 10, y: 20, z: 5 },
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PositionComponent = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}
