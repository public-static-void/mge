#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::derived_stats::DerivedStatsSystem;
use engine_core::systems::equipment_effect_aggregation::EquipmentEffectAggregationSystem;
use engine_core::systems::stat_calculation::StatCalculationSystem;
use serde_json::json;
use std::time::Instant;

/// Tests the full stat pipeline: BaseStats + EquipmentEffects → Stats → DerivedStats
///
/// Pipeline:
///   StatCalculationSystem: Stats[k] = (BaseStats[k] || 0) + (EquipmentEffects[k] || 0)
///   DerivedStatsSystem:
///     DerivedStats.MaxHP = 100 + stats.constitution * 10
///     DerivedStats.MeleeDamage = 1.0 + stats.strength * 0.5
///     DerivedStats.CritChance = 0.05 + stats.intelligence * 0.005
#[test]
fn test_full_stat_pipeline_basic() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentEffectAggregationSystem);
    world.register_system(StatCalculationSystem);
    world.register_system(DerivedStatsSystem);

    let eid = world.spawn_entity();

    // Set BaseStats with all three documented stats
    world
        .set_component(
            eid,
            "BaseStats",
            json!({
                "strength": 10.0,
                "dexterity": 8.0,
                "intelligence": 6.0,
                "constitution": 5.0
            }),
        )
        .unwrap();

    // Run the full pipeline
    world
        .run_system("EquipmentEffectAggregationSystem")
        .unwrap();
    world.run_system("StatCalculationSystem").unwrap();
    world.run_system("DerivedStatsSystem").unwrap();

    // Stats should reflect BaseStats (no equipment effects)
    let stats = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats["strength"].as_f64().unwrap(), 10.0);
    assert_eq!(stats["dexterity"].as_f64().unwrap(), 8.0);
    assert_eq!(stats["intelligence"].as_f64().unwrap(), 6.0);
    assert_eq!(stats["constitution"].as_f64().unwrap(), 5.0);

    // DerivedStats should be computed from Stats
    let derived = world.get_component(eid, "DerivedStats").unwrap();
    assert_eq!(
        derived["MaxHP"].as_f64().unwrap(),
        100.0 + 5.0 * 10.0,
        "MaxHP = 100 + constitution*10"
    );
    assert_eq!(
        derived["MeleeDamage"].as_f64().unwrap(),
        1.0 + 10.0 * 0.5,
        "MeleeDamage = 1.0 + strength*0.5"
    );
    assert_eq!(
        derived["CritChance"].as_f64().unwrap(),
        0.05 + 6.0 * 0.005,
        "CritChance = 0.05 + intelligence*0.005"
    );
}

/// Tests that EquipmentEffects are correctly aggregated into Stats.
#[test]
fn test_stat_pipeline_with_equipment_effects() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentEffectAggregationSystem);
    world.register_system(StatCalculationSystem);
    world.register_system(DerivedStatsSystem);

    let eid = world.spawn_entity();

    // Set BaseStats
    world
        .set_component(
            eid,
            "BaseStats",
            json!({
                "strength": 5.0,
                "intelligence": 3.0,
                "constitution": 2.0
            }),
        )
        .unwrap();

    // Set EquipmentEffects directly (simulating what the aggregation system would produce)
    world
        .set_component(
            eid,
            "EquipmentEffects",
            json!({
                "strength": 3.0,
                "dexterity": 1.0
            }),
        )
        .unwrap();

    // Run stat pipeline
    world.run_system("StatCalculationSystem").unwrap();
    world.run_system("DerivedStatsSystem").unwrap();

    // Stats = BaseStats + EquipmentEffects (no aggregation system needed for this test)
    let stats = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats["strength"].as_f64().unwrap(), 8.0); // 5 + 3
    assert_eq!(stats["dexterity"].as_f64().unwrap(), 1.0); // 0 + 1 (dexterity not in BaseStats)
    assert_eq!(stats["intelligence"].as_f64().unwrap(), 3.0); // 3 + 0

    // DerivedStats computed from final Stats
    let derived = world.get_component(eid, "DerivedStats").unwrap();
    assert_eq!(derived["MaxHP"].as_f64().unwrap(), 100.0 + 2.0 * 10.0);
    assert_eq!(derived["MeleeDamage"].as_f64().unwrap(), 1.0 + 8.0 * 0.5);
    assert_eq!(derived["CritChance"].as_f64().unwrap(), 0.05 + 3.0 * 0.005);
}

/// Tests stat pipeline with no BaseStats (entity should be skipped).
#[test]
fn test_stat_pipeline_no_base_stats() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(StatCalculationSystem);
    world.register_system(DerivedStatsSystem);

    let eid = world.spawn_entity();

    // Entity has no BaseStats component — pipeline should skip it
    world.run_system("StatCalculationSystem").unwrap();
    world.run_system("DerivedStatsSystem").unwrap();

    // No Stats should be set
    assert!(world.get_component(eid, "Stats").is_none());
    assert!(world.get_component(eid, "DerivedStats").is_none());
}

/// Tests DerivedStatsSystem with null/zero Stats.
#[test]
fn test_derived_stats_with_defaults() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(DerivedStatsSystem);

    let eid = world.spawn_entity();

    // Set Stats with only some values populated
    world
        .set_component(
            eid,
            "Stats",
            json!({
                "strength": 0.0,
                "constitution": 0.0,
                "intelligence": 0.0
            }),
        )
        .unwrap();

    world.run_system("DerivedStatsSystem").unwrap();

    let derived = world.get_component(eid, "DerivedStats").unwrap();
    // Stat values of 0 should produce base-line derived values
    assert_eq!(derived["MaxHP"].as_f64().unwrap(), 100.0);
    assert_eq!(derived["MeleeDamage"].as_f64().unwrap(), 1.0);
    assert_eq!(derived["CritChance"].as_f64().unwrap(), 0.05);
}

/// Tests DerivedStatsSystem with missing stats (falls back to defaults).
#[test]
fn test_derived_stats_missing_stats() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(DerivedStatsSystem);

    let eid = world.spawn_entity();

    // Stats component exists but is empty
    world.set_component(eid, "Stats", json!({})).unwrap();

    world.run_system("DerivedStatsSystem").unwrap();

    let derived = world.get_component(eid, "DerivedStats").unwrap();
    // Missing stats default to 0, producing base-line derived values
    assert_eq!(derived["MaxHP"].as_f64().unwrap(), 100.0);
    assert_eq!(derived["MeleeDamage"].as_f64().unwrap(), 1.0);
    assert_eq!(derived["CritChance"].as_f64().unwrap(), 0.05);
}

/// Tests that DerivedStatsSystem skips entities without a Stats component.
#[test]
fn test_derived_stats_no_stats_component() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(DerivedStatsSystem);

    let eid = world.spawn_entity();
    // Entity has no Stats component

    world.run_system("DerivedStatsSystem").unwrap();
    assert!(world.get_component(eid, "DerivedStats").is_none());
}

/// Tests that DerivedStatsSystem updates when Stats change (reactivity).
#[test]
fn test_derived_stats_reactivity() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(DerivedStatsSystem);

    let eid = world.spawn_entity();

    // Set initial Stats
    world
        .set_component(
            eid,
            "Stats",
            json!({
                "strength": 10.0,
                "constitution": 5.0,
                "intelligence": 3.0
            }),
        )
        .unwrap();

    world.run_system("DerivedStatsSystem").unwrap();

    let derived = world.get_component(eid, "DerivedStats").unwrap();
    assert_eq!(derived["MeleeDamage"].as_f64().unwrap(), 1.0 + 10.0 * 0.5);

    // Update Stats
    world
        .set_component(
            eid,
            "Stats",
            json!({
                "strength": 20.0,
                "constitution": 5.0,
                "intelligence": 3.0
            }),
        )
        .unwrap();

    world.run_system("DerivedStatsSystem").unwrap();

    let derived = world.get_component(eid, "DerivedStats").unwrap();
    assert_eq!(derived["MeleeDamage"].as_f64().unwrap(), 1.0 + 20.0 * 0.5);
}

/// Tests that all pipeline systems run sequentially in correct order.
#[test]
fn test_stat_pipeline_sequential_systems() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentEffectAggregationSystem);
    world.register_system(StatCalculationSystem);
    world.register_system(DerivedStatsSystem);

    let eid = world.spawn_entity();

    world
        .set_component(
            eid,
            "BaseStats",
            json!({
                "strength": 8.0,
                "constitution": 3.0,
                "intelligence": 4.0
            }),
        )
        .unwrap();

    world
        .set_component(
            eid,
            "EquipmentEffects",
            json!({
                "strength": 2.0
            }),
        )
        .unwrap();

    // Run each system in pipeline order
    world
        .run_system("EquipmentEffectAggregationSystem")
        .unwrap();
    world.run_system("StatCalculationSystem").unwrap();
    world.run_system("DerivedStatsSystem").unwrap();

    let stats = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats["strength"].as_f64().unwrap(), 10.0); // 8 + 2

    let derived = world.get_component(eid, "DerivedStats").unwrap();
    assert_eq!(derived["MaxHP"].as_f64().unwrap(), 100.0 + 3.0 * 10.0);
    assert_eq!(derived["MeleeDamage"].as_f64().unwrap(), 1.0 + 10.0 * 0.5);
    assert_eq!(derived["CritChance"].as_f64().unwrap(), 0.05 + 4.0 * 0.005);
}

/// Performance benchmark: verifies stat calculation for 100 entities completes quickly.
/// This serves as a proxy for NFR003 (stat calculation should not introduce slowdown).
/// Threshold: 100 entities with BaseStats + EquipmentEffects should complete in < 2 seconds.
#[test]
fn test_stat_calculation_performance_benchmark() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentEffectAggregationSystem);
    world.register_system(StatCalculationSystem);

    // Create 100 entities with BaseStats and EquipmentEffects
    let entity_ids: Vec<u32> = (0..100)
        .map(|i| {
            let eid = world.spawn_entity();
            world
                .set_component(
                    eid,
                    "BaseStats",
                    json!({
                        "strength": 5.0 + (i as f64 * 0.1),
                        "dexterity": 4.0,
                        "intelligence": 3.0,
                        "constitution": 2.0,
                    }),
                )
                .unwrap();
            world
                .set_component(
                    eid,
                    "EquipmentEffects",
                    json!({
                        "strength": 3.0,
                        "charisma": 1.0,
                    }),
                )
                .unwrap();
            eid
        })
        .collect();

    // Benchmark: run both pipeline systems
    let start = Instant::now();
    for _ in 0..5 {
        world
            .run_system("EquipmentEffectAggregationSystem")
            .unwrap();
        world.run_system("StatCalculationSystem").unwrap();
    }
    let elapsed = start.elapsed();

    // Verify correctness on a sample entity
    let stats = world.get_component(entity_ids[0], "Stats").unwrap();
    assert!(
        (stats["strength"].as_f64().unwrap() - 8.0).abs() < f64::EPSILON,
        "Stats should be BaseStats + EquipmentEffects"
    );

    // All 100 entities should have Stats computed
    for &eid in &entity_ids {
        let stats = world.get_component(eid, "Stats").unwrap();
        assert!(stats.get("strength").and_then(|v| v.as_f64()).is_some());
    }

    // Performance assertion: 5 iterations over 100 entities should complete in < 2 seconds
    assert!(
        elapsed.as_secs_f64() < 2.0,
        "Stat pipeline benchmark took {:.3}s (threshold: 2.0s)",
        elapsed.as_secs_f64()
    );
}
