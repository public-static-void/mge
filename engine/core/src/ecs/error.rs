use semver::Version;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Component not registered")]
    UnregisteredComponent,
    #[error("Invalid schema")]
    InvalidSchema,
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(Version),
    #[error("Deserialization failed: {0}")]
    DeserializationError(#[from] bson::de::Error),
    #[error("Data format mismatch")]
    DataFormatError,
}
