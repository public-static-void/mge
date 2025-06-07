use semver::Version;
use thiserror::Error;

/// Errors that can occur during component registry operations.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// The requested component is not registered in the registry.
    #[error("Component not registered")]
    UnregisteredComponent,
    /// The component schema is invalid.
    #[error("Invalid schema")]
    InvalidSchema,
    /// Serialization or deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Errors that can occur during component migration.
#[derive(Debug, Error)]
pub enum MigrationError {
    /// The version being migrated from is not supported.
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(Version),
    /// Deserialization of data failed.
    #[error("Deserialization failed: {0}")]
    DeserializationError(#[from] bson::de::Error),
    /// The data format does not match the expected schema.
    #[error("Data format mismatch")]
    DataFormatError,
}
