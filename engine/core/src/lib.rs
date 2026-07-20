//! Core engine library for the Modular Game Engine.
//!
//! Exposes ECS and mode management modules.

/// Config module
pub mod config;
/// ECS module
pub mod ecs;
/// Faction and reputation system
pub mod faction;
/// Loot table system
pub mod loot;
/// Map module
pub mod map;
/// Material property lookup and entity material management
pub mod material;
/// Modes module
pub mod modes;
/// Mods module
pub mod mods;
/// Plugins module
pub mod plugins;
/// Presentation module
pub mod presentation;
/// Systems module
pub mod systems;
/// Tech tree and research system
pub mod tech_tree;
/// Worldgen module
pub mod worldgen;

pub use ecs::World;
pub use ecs::components::{Happiness, Health, Inventory, Position};
pub use modes::GameMode as Mode;
