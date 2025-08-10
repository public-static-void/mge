//! ECS (Entity Component System) core module.
//!
//! Exposes core ECS types, schema support, and error handling.

/// Assets
pub mod assets;
/// Components
pub mod components;
mod error;
/// Events
pub mod event;
/// Event bus registry
pub mod event_bus_registry;
/// Event logger
pub mod event_logger;
/// Component registry
pub mod registry;
/// Schemas
pub mod schema;
/// Systems
pub mod system;
/// World
pub mod world;

pub use components::{Health, Position};
pub use error::{MigrationError, RegistryError};
pub use registry::{Component, ComponentRegistry};
pub use schema::ComponentSchema;
pub use world::World;
