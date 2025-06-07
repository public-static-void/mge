use crate::ecs::error::MigrationError;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Position for any map topology (square, hex, region, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Position {
    Square { x: i32, y: i32, z: i32 },
    Hex { q: i32, r: i32, z: i32 },
    Region { id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PositionComponent {
    pub pos: Position,
}

/// Legacy struct for migration from version 1.0.0
#[derive(Deserialize)]
pub struct LegacyPosition {
    pub x: f32,
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
