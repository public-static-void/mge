use crate::ecs::error::MigrationError;
use crate::ecs::Component;
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct Position {
    #[schemars(description = "X coordinate in world units")]
    pub x: f32,

    #[schemars(description = "Y coordinate in world units")]
    pub y: f32,
}

impl Component for Position {
    fn generate_schema() -> Option<schemars::schema::RootSchema> {
        Some(schemars::schema_for!(Position))
    }

    fn version() -> Version {
        Version::parse("2.1.0").unwrap() // Bump version
    }

    fn migrate(from_version: Version, data: &[u8]) -> Result<Self, MigrationError> {
        match from_version.major {
            1 => {
                // Legacy format had different field names
                #[derive(Deserialize)]
                struct LegacyPosition {
                    pos_x: f32,
                    pos_y: f32,
                }

                let legacy = bson::from_slice::<LegacyPosition>(data)?;
                Ok(Self {
                    x: legacy.pos_x,
                    y: legacy.pos_y,
                })
            }
            2 => bson::from_slice(data).map_err(Into::into),
            _ => Err(MigrationError::UnsupportedVersion(from_version)),
        }
    }
}
