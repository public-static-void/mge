//! ECS (Entity Component System) core module.
//!
//! Exposes core ECS types, schema support, and error handling.

/// Assets
pub mod assets;
/// Components
pub mod components;
/// Equipment set registry
pub mod equipment_set;
mod error;
/// Events
pub mod event;
/// Event bus registry
pub mod event_bus_registry;
/// Event logger
pub mod event_logger;
/// Item definition registry
pub mod item;
/// Component registry
pub mod registry;
/// Schemas
pub mod schema;
/// Systems
pub mod system;
/// Unit template registry
pub mod template;
/// World
pub mod world;

pub use components::{Health, Position};
pub use equipment_set::{EquipmentSet, EquipmentSetRegistry};
pub use error::{MigrationError, RegistryError};
pub use item::ItemRegistry;
pub use registry::{Component, ComponentRegistry};
pub use schema::ComponentSchema;
pub use template::{UnitTemplate, UnitTemplateRegistry};
pub use world::World;
