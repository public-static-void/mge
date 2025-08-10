//! ECS component definitions.
//!
//! This module re-exports all built-in components.

mod corpse;
mod decay;
mod happiness;
mod health;
mod inventory;
/// Position component
pub mod position;

pub use self::corpse::Corpse;
pub use self::decay::Decay;
pub use self::happiness::Happiness;
pub use self::health::Health;
pub use self::inventory::Inventory;
pub use self::position::Position;
