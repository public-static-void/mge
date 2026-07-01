//! System modules
//!
//! Systems are functions that run on the ECS world and can be used to modify the state of the world.

/// Body and equipment synchronization system
pub mod body_equipment_sync;
/// Death and decay system
pub mod death_decay;
/// Procedural dungeon generation
pub mod dungeon;
/// Economic system
pub mod economic;
/// Equipment effect aggregation system
pub mod equipment_effect_aggregation;
/// Equipment logic system
pub mod equipment_logic;
/// Faction reputation system
pub mod faction_reputation;
/// Fog-of-war update system
pub mod fog;
/// Field-of-view update system
pub mod fov;
/// Inventory system
pub mod inventory;
/// Job system
pub mod job;
/// Movement system
pub mod movement_system;
/// Stat calculation system
pub mod stat_calculation;
