//! Core engine library for the Modular Game Engine.
//!
//! Exposes ECS and mode management modules.

/// Config module
pub mod config;
/// ECS module
pub mod ecs;
/// Map module
pub mod map;
/// Modes module
pub mod modes;
/// Mods module
pub mod mods;
/// Plugins module
pub mod plugins;
/// Presentation module
pub mod presentation;
/// Loot table system
pub mod loot;
/// Systems module
pub mod systems;
/// Worldgen module
pub mod worldgen;

pub use ecs::World;
pub use ecs::components::{Happiness, Health, Inventory, Position};
pub use modes::GameMode as Mode;
