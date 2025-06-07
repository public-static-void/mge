use crate::ecs::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Position for any map topology (square, hex, region, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum Position {
    Square { x: i32, y: i32, z: i32 },
    Hex { q: i32, r: i32, z: i32 },
    Region { id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PositionComponent {
    pub pos: Position,
}

impl Component for PositionComponent {
    fn generate_schema() -> Option<schemars::schema::Schema> {
        Some(schemars::schema_for!(PositionComponent))
    }

    fn version() -> semver::Version {
        semver::Version::parse("3.0.0").unwrap()
    }

    fn migrate(
        from_version: semver::Version,
        data: &[u8],
    ) -> Result<Self, crate::ecs::error::MigrationError>
    where
        Self: Sized + serde::de::DeserializeOwned,
    {
        Err(crate::ecs::error::MigrationError::UnsupportedVersion(
            from_version,
        ))
    }
}
