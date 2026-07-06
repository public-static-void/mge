//! System modules
//!
//! Systems are functions that run on the ECS world and can be used to modify the state of the world.

/// Body and equipment synchronization system
pub mod body_equipment_sync;
/// Death and decay system
pub mod death_decay;
/// Derived stats calculation system
pub mod derived_stats;
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
/// Research system
pub mod research;
/// Stat calculation system
pub mod stat_calculation;

/// Deterministic system execution order per specification R011.
///
/// Systems execute in this exact sequence when `run_all_systems` is called.
/// Systems not in this array execute after in registration order (for extensibility).
pub const SYSTEM_EXECUTION_ORDER: &[&str] = &[
    "EquipmentLogicSystem",
    "EquipmentEffectAggregationSystem",
    "BodyEquipmentSyncSystem",
    "StatCalculationSystem",
    "DerivedStatsSystem",
    "ResearchSystem",
    "JobSystem",
    "FactionReputationSystem",
    "FovUpdateSystem",
    "ProcessDeaths",
    "ProcessDecay",
];

/// Orders system names according to the deterministic execution order.
///
/// Systems in [`SYSTEM_EXECUTION_ORDER`] are placed at their specified positions.
/// Systems not in the ordering list are appended after in their original relative order.
pub fn order_systems(system_names: &[String]) -> Vec<String> {
    let mut remaining: std::collections::HashSet<&str> =
        system_names.iter().map(|s| s.as_str()).collect();
    let mut ordered: Vec<String> = Vec::with_capacity(system_names.len());

    for &name in SYSTEM_EXECUTION_ORDER {
        if remaining.remove(name) {
            ordered.push(name.to_string());
        }
    }

    // Append remaining systems in their original registration order for extensibility
    for name in system_names {
        if remaining.contains(name.as_str()) {
            ordered.push(name.clone());
        }
    }

    ordered
}
