//! ECS component definitions.
//!
//! This module re-exports all built-in components.

mod colony_happiness;
mod corpse;
mod decay;
mod health;
mod position;
mod roguelike_inventory;

pub use self::colony_happiness::ColonyHappiness;
pub use self::corpse::Corpse;
pub use self::decay::Decay;
pub use self::health::Health;
pub use self::position::Position;
pub use self::roguelike_inventory::RoguelikeInventory;
