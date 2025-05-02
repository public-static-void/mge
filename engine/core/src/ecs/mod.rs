mod components;
mod error;
pub(crate) mod registry;

pub use components::{Health, Position};
pub use error::{MigrationError, RegistryError};
pub use registry::{Component, ComponentRegistry};

#[derive(Debug)]
pub struct ComponentSchema {
    pub name: String,
    pub schema: Option<schemars::schema::RootSchema>,
}
