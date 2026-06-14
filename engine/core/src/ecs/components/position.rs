use crate::ecs::error::MigrationError;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Position for any map topology (square, hex, province, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Position {
    /// Square topology
    Square {
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
        /// Z coordinate
        z: i32,
    },
    /// Hexagonal topology
    Hex {
        /// q coordinate
        q: i32,
        /// r coordinate
        r: i32,
        /// z coordinate
        z: i32,
    },
    /// Province topology
    Province {
        /// Province ID
        id: String,
    },
}

/// Position component
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PositionComponent {
    /// Position
    pub pos: Position,
}

/// Legacy struct for migration from version 1.0.0
#[derive(Deserialize)]
pub struct LegacyPosition {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
}

impl crate::ecs::Component for PositionComponent {
    fn generate_schema() -> Option<schemars::Schema> {
        Some(schemars::schema_for!(PositionComponent))
    }

    fn version() -> Version {
        Version::parse("3.0.0").unwrap()
    }

    fn migrate(from_version: Version, data: &[u8]) -> Result<Self, MigrationError>
    where
        Self: Sized + serde::de::DeserializeOwned,
    {
        if from_version == Version::parse("1.0.0").unwrap() {
            let legacy = bson::from_slice::<LegacyPosition>(data)
                .map_err(MigrationError::DeserializationError)?;
            Ok(PositionComponent {
                pos: Position::Square {
                    x: legacy.x as i32,
                    y: legacy.y as i32,
                    z: 0,
                },
            })
        } else {
            Err(MigrationError::UnsupportedVersion(from_version))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Component;
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
    fn test_migrate_unsupported_v2() {
        let legacy = TestLegacyPosition { x: 0.0, y: 0.0 };
        let data = bson::to_vec(&legacy).unwrap();
        let result = PositionComponent::migrate(Version::parse("2.0.0").unwrap(), &data);
        assert!(matches!(result, Err(MigrationError::UnsupportedVersion(_))));
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
}
