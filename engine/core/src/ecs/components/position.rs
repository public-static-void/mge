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
