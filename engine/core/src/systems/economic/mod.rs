pub mod loader;
pub mod recipe;
pub mod resource;
pub mod system;

pub use loader::load_recipes_from_dir;
pub use recipe::{Recipe, ResourceAmount};
pub use system::EconomicSystem;
