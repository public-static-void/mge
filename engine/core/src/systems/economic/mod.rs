//! Economic system
//!
//!The economic system is responsible for the resource management.

/// Recipes loader
pub mod loader;
/// Recipes
pub mod recipe;
/// Resources
pub mod resource;
/// Economic system
pub mod system;

pub use loader::load_recipes_from_dir;
pub use recipe::{Recipe, ResourceAmount};
pub use system::EconomicSystem;
