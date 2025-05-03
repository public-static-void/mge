//! ECS (Entity Component System) core module.
//!
//! Exposes core ECS types, schema support, and error handling.

mod components;
mod error;
pub(crate) mod registry;

pub use components::{Health, Position};
pub use error::{MigrationError, RegistryError};
pub use registry::{Component, ComponentRegistry};

/// Represents the JSON schema for a component, used for dynamic registration and validation.
#[derive(Debug)]
pub struct ComponentSchema {
    pub name: String,
    pub schema: Option<schemars::schema::RootSchema>,
}
