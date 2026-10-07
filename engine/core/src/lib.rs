//! Core engine library for the Modular Game Engine.
//!
//! Exposes ECS and mode management modules.

/// Shared bridge-domain contract for the scripting API-module registries
pub mod api_registry;
/// Config module
pub mod config;
/// Diplomacy and faction-pair relationship system
pub mod diplomacy;
/// ECS module
pub mod ecs;
/// Faction and reputation system
pub mod faction;
/// Loot table system
pub mod loot;
/// Procedural history and lore (world chronicle read-model)
pub mod lore;
/// Map module
pub mod map;
/// Material property lookup and entity material management
pub mod material;
/// Modes module
pub mod modes;
/// Mods module
pub mod mods;
/// Event-driven narrative engine (scenarios, decision events)
pub mod narrative;
/// Plugins module
pub mod plugins;
/// Presentation module
pub mod presentation;
/// Supply-link lifecycle (validated per-tick pushes between stockpiles)
pub mod supply;
/// Systems module
pub mod systems;
/// Tech tree and research system
pub mod tech_tree;
/// Atomic inter-stockpile trade primitives and treaty queries
pub mod trade;
/// Worldgen module
pub mod worldgen;

pub use ecs::World;
pub use ecs::components::{Happiness, Health, Inventory, Position};
pub use modes::GameMode as Mode;
