//! System modules
//!
//! Systems are functions that run on the ECS world and can be used to modify the state of the world.

/// Body and equipment synchronization system
pub mod body_equipment_sync;
/// Death and decay system
pub mod death_decay;
/// Economic system
pub mod economic;
/// Equipment effect aggregation system
pub mod equipment_effect_aggregation;
/// Equipment logic system
pub mod equipment_logic;
/// Inventory system
pub mod inventory;
/// Job system
pub mod job;
/// Movement system
pub mod movement_system;
/// Stat calculation system
pub mod stat_calculation;
