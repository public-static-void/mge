//! ECS (Entity Component System) core module.
//!
//! Exposes core ECS types, schema support, and error handling.

pub mod components;
mod error;
pub mod event;
pub mod registry;
pub mod schema;
pub use schema::ComponentSchema;
pub mod event_bus_registry;
pub mod event_logger;
pub mod system;
pub mod world;
pub use components::{Health, Position};
pub use error::{MigrationError, RegistryError};
pub use registry::{Component, ComponentRegistry};
pub use world::World;
