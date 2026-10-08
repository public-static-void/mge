//! System modules
//!
//! Systems are functions that run on the ECS world and can be used to modify the state of the world.

/// Body and equipment synchronization system
pub mod body_equipment_sync;
/// Body part damage distribution system
pub mod body_part_damage;
/// Cellular-caves map generation (second swappable mapgen algorithm)
pub mod cellular_caves;
/// Building construction system
pub mod construction;
/// Autonomous per-entity upkeep consumption system
pub mod consumption;
/// Crafting system (tool-gated, material/quality-aware, skill-gated Item production)
pub mod crafting;
/// Death and decay system
pub mod death_decay;
/// Derived stats calculation system
pub mod derived_stats;
/// Diplomacy treaty-expiry system
pub mod diplomacy;
/// Procedural dungeon generation
pub mod dungeon;
/// Economic system
pub mod economic;
/// Ecosystem simulation system (deterministic wildlife graze/wander FSM)
pub mod ecosystem;
/// Enemy AI behavior system (deterministic FSM: idle/patrol/chase/attack/flee)
pub mod enemy_behavior;
/// Equipment effect aggregation system
pub mod equipment_effect_aggregation;
/// Equipment logic system
pub mod equipment_logic;
/// Faction reputation system
pub mod faction_reputation;
/// Fluid simulation system (water, magma)
pub mod fluid;
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
/// Narrative incident-director system (scenario trigger evaluation)
pub mod narrative;
/// Noise propagation system
pub mod noise;
/// Research system
pub mod research;
/// Stat calculation system
pub mod stat_calculation;
/// Per-tick supply push between linked stockpiles
pub mod supply;
/// Temperature system (deterministic global ambient + per-body heat exchange)
pub mod temperature;
/// Vehicle carrier system (embark/disembark + mounted co-movement)
pub mod vehicle;
/// Weather system (deterministic global weather state)
pub mod weather;
/// Zone state validation system
pub mod zone;

/// Deterministic system execution order per specification R011.
///
/// Systems execute in this exact sequence when `run_all_systems` is called.
/// Systems not in this array execute after in registration order (for extensibility).
pub const SYSTEM_EXECUTION_ORDER: &[&str] = &[
    "BodyPartDamageSystem",
    "EquipmentLogicSystem",
    "EquipmentEffectAggregationSystem",
    "BodyEquipmentSyncSystem",
    "StatCalculationSystem",
    "DerivedStatsSystem",
    "ResearchSystem",
    "JobSystem",
    "EconomicSystem",
    "ConsumptionSystem",
    "SupplySystem",
    "CraftingSystem",
    "ConstructionSystem",
    "ZoneSystem",
    "FactionReputationSystem",
    "DiplomacySystem",
    "NarrativeSystem",
    "FluidSimulationSystem",
    "WeatherSystem",
    "TemperatureSystem",
    "FovUpdateSystem",
    "NoiseSystem",
    "EnemyBehaviorSystem",
    "EcosystemSystem",
    "MovementSystem",
    "VehicleSystem",
    "ProcessDeaths",
    "ProcessDecay",
];

/// Orders system names according to the deterministic execution order.
///
/// Systems in [`SYSTEM_EXECUTION_ORDER`] are placed at their specified positions.
/// Systems not in the ordering list are appended after in their original relative order.
pub fn order_systems(system_names: &[String]) -> Vec<String> {
    order_systems_with_dependencies(system_names, &std::collections::HashMap::new())
}

/// Orders system names with dependency awareness for unlisted systems.
///
/// Systems in [`SYSTEM_EXECUTION_ORDER`] keep their pack positions first.
/// Remaining systems follow topological order over `dependencies`, so a new
/// system with a declared dependency slots after it without editing the
/// closed list. Only edges between remaining systems constrain the tie-break:
/// dependencies on listed systems are already satisfied (the pack runs
/// first) and unknown names are ignored. Registration order — the input
/// slice order — breaks remaining ties. Cyclic leftovers keep input order
/// instead of erroring: this is a tie-break, not a scheduler.
pub fn order_systems_with_dependencies(
    system_names: &[String],
    dependencies: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<String> {
    use std::collections::{BTreeSet, HashMap, HashSet};

    let listed: HashSet<&str> = SYSTEM_EXECUTION_ORDER.iter().copied().collect();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut ordered: Vec<String> = Vec::with_capacity(system_names.len());

    for &name in SYSTEM_EXECUTION_ORDER {
        if system_names.iter().any(|n| n == name) && seen.insert(name) {
            ordered.push(name.to_string());
        }
    }

    // Unlisted systems, deduplicated, in registration (input) order.
    let unlisted: Vec<&str> = system_names
        .iter()
        .map(|s| s.as_str())
        .filter(|n| !listed.contains(n) && seen.insert(*n))
        .collect();
    let position: HashMap<&str, usize> =
        unlisted.iter().enumerate().map(|(i, n)| (*n, i)).collect();

    // Kahn's algorithm over the unlisted subgraph, input order as tie-break.
    let mut indegree = vec![0usize; unlisted.len()];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); unlisted.len()];
    for (i, name) in unlisted.iter().enumerate() {
        if let Some(deps) = dependencies.get(*name) {
            for dep in deps {
                if let Some(&j) = position.get(dep.as_str()) {
                    dependents[j].push(i);
                    indegree[i] += 1;
                }
            }
        }
    }
    let mut ready: BTreeSet<usize> = indegree
        .iter()
        .enumerate()
        .filter(|(_, d)| **d == 0)
        .map(|(i, _)| i)
        .collect();
    let mut emitted = vec![false; unlisted.len()];
    while let Some(&i) = ready.iter().next() {
        ready.remove(&i);
        emitted[i] = true;
        ordered.push(unlisted[i].to_string());
        for &k in &dependents[i] {
            indegree[k] -= 1;
            if indegree[k] == 0 {
                ready.insert(k);
            }
        }
    }
    // Cyclic leftovers keep registration order instead of erroring.
    for (i, name) in unlisted.iter().enumerate() {
        if !emitted[i] {
            ordered.push((*name).to_string());
        }
    }

    ordered
}
