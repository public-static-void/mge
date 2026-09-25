//! Integration tests for the EcosystemSystem FSM (M2 core + M3 full FSM).
//!
//! Covers registration/ordering (AC003), no-map no-op (AC004), graze gain
//! (AC005), season ratios (AC006), wander movement + determinism (AC007),
//! flee trigger + recovery (AC008), activity-phase rest (AC009), birth and
//! temperature gate (AC010, AC011), predation (AC012), and event payloads
//! (AC015).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::ecosystem::EcosystemSystem;
use engine_core::systems::{SYSTEM_EXECUTION_ORDER, order_systems};
use serde_json::json;

/// Open square plane with 8-directional adjacency.
fn open_plane(size: i32) -> Map {
    let mut grid = SquareGridMap::new();
    for x in -size..=size {
        for y in -size..=size {
            grid.add_cell(x, y, 0);
        }
    }
    for x in -size..=size {
        for y in -size..=size {
            for dx in [-1, 0, 1] {
                for dy in [-1, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx >= -size && nx <= size && ny >= -size && ny <= size {
                        grid.add_neighbor((x, y, 0), (nx, ny, 0));
                    }
                }
            }
        }
    }
    Map::new(Box::new(grid))
}

fn ecosystem_world() -> World {
    let mut world = make_test_world();
    world.map = Some(open_plane(10));
    // Midday so diurnal entities follow the graze/wander loop by default;
    // night-phase tests set the hour explicitly.
    world.time_of_day.hour = 12;
    world
}

fn spawn_wildlife(
    world: &mut World,
    x: i32,
    y: i32,
    state: &str,
    satiety: f64,
    diet: Option<&str>,
) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Wildlife",
            json!({
                "state": state,
                "satiety": satiety,
                "reproduction_cooldown": 0,
                "flee_ticks": 0,
                "rest_ticks": 0,
            }),
        )
        .unwrap();
    world
        .set_component(
            eid,
            "Position",
            json!({"pos": {"Square": {"x": x, "y": y, "z": 0}}}),
        )
        .unwrap();
    if let Some(diet) = diet {
        world
            .set_component(
                eid,
                "Species",
                json!({
                    "diet": diet,
                    "graze_nutrition_rate": 0.2,
                    "metabolism_rate": 0.05,
                    "reproduction_threshold": 0.8,
                    "reproduction_cooldown_ticks": 10,
                    "litter_size": 1,
                    "detection_range": 6,
                    "noise_flee_threshold": 0.5,
                    "activity": "diurnal",
                }),
            )
            .unwrap();
    }
    eid
}

fn satiety_of(world: &World, eid: u32) -> f64 {
    world
        .get_component(eid, "Wildlife")
        .unwrap()
        .get("satiety")
        .and_then(|v| v.as_f64())
        .unwrap()
}

fn cell_of(world: &World, eid: u32) -> CellKey {
    CellKey::from_position(world.get_component(eid, "Position").unwrap()).unwrap()
}

fn state_of(world: &World, eid: u32) -> String {
    world
        .get_component(eid, "Wildlife")
        .unwrap()
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string()
}

/// Spawn a live threat: EnemyAI in chase state with Health and Position.
fn spawn_threat(world: &mut World, x: i32, y: i32) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "EnemyAI",
            json!({
                "state": "chase",
                "alert_level": 1.0,
                "detection_range": 8,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": null,
                "target_entity": null,
            }),
        )
        .unwrap();
    world
        .set_component(
            eid,
            "Position",
            json!({"pos": {"Square": {"x": x, "y": y, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(eid, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();
    eid
}

/// Overwrite the Species component with a custom activity pattern.
fn set_activity(world: &mut World, eid: u32, activity: &str) {
    world
        .set_component(
            eid,
            "Species",
            json!({
                "diet": "herbivore",
                "graze_nutrition_rate": 0.2,
                "metabolism_rate": 0.05,
                "reproduction_threshold": 0.8,
                "reproduction_cooldown_ticks": 10,
                "litter_size": 1,
                "detection_range": 6,
                "noise_flee_threshold": 0.5,
                "activity": activity,
            }),
        )
        .unwrap();
}

fn path_steps_between(world: &World, a: &CellKey, b: &CellKey) -> usize {
    world
        .find_path(a, b)
        .map(|r| r.path.len().saturating_sub(1))
        .unwrap()
}

#[test]
fn test_ecosystem_registration_and_ordering() {
    let system = EcosystemSystem;
    assert_eq!(system.name(), "EcosystemSystem");
    assert_eq!(system.dependencies(), &["FovUpdateSystem", "NoiseSystem"]);
    assert!(SYSTEM_EXECUTION_ORDER.contains(&"EcosystemSystem"));

    let ordered = order_systems(&[
        "ProcessDeaths".to_string(),
        "EcosystemSystem".to_string(),
        "EnemyBehaviorSystem".to_string(),
    ]);
    assert_eq!(
        ordered,
        vec![
            "EnemyBehaviorSystem".to_string(),
            "EcosystemSystem".to_string(),
            "ProcessDeaths".to_string(),
        ]
    );
}

#[test]
fn test_ecosystem_no_map_noop() {
    let mut world = make_test_world();
    assert!(world.map.is_none());
    let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));
    let before = world.get_component(eid, "Wildlife").unwrap().clone();

    EcosystemSystem.run(&mut world);

    assert_eq!(world.get_component(eid, "Wildlife").unwrap(), &before);
}

#[test]
fn test_ecosystem_graze_gain_exact_and_stationary() {
    let mut world = ecosystem_world();
    world.time_of_day.day = 0; // spring: modifier 1.0
    let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));

    EcosystemSystem.run(&mut world);

    let satiety = satiety_of(&world, eid);
    assert!(
        (satiety - 0.7).abs() < 1e-9,
        "spring gain must equal graze_nutrition_rate * 1.0, got {satiety}"
    );
    assert_eq!(cell_of(&world, eid), CellKey::Square { x: 0, y: 0, z: 0 });
}

#[test]
fn test_ecosystem_season_ratios() {
    // day 0 = spring (1.0), day 60 = autumn (0.6), day 90 = winter (0.25)
    let mut gains = Vec::new();
    for day in [0u64, 60, 90] {
        let mut world = ecosystem_world();
        world.time_of_day.day = day;
        let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.2, Some("herbivore"));
        EcosystemSystem.run(&mut world);
        gains.push(satiety_of(&world, eid) - 0.2);
    }
    assert!((gains[0] - 0.2).abs() < 1e-9, "spring gain {gains:?}");
    assert!(
        (gains[1] / gains[0] - 0.6).abs() < 1e-9,
        "autumn ratio {gains:?}"
    );
    assert!(
        (gains[2] / gains[0] - 0.25).abs() < 1e-9,
        "winter ratio {gains:?}"
    );
}

#[test]
fn test_ecosystem_wander_moves_and_costs() {
    let mut world = ecosystem_world();
    let eid = spawn_wildlife(&mut world, 0, 0, "wander", 0.5, Some("herbivore"));
    let before = cell_of(&world, eid);

    EcosystemSystem.run(&mut world);

    let after = cell_of(&world, eid);
    assert_ne!(after, before, "wander must move every tick");
    assert!(
        world
            .map
            .as_ref()
            .unwrap()
            .neighbors(&before)
            .contains(&after),
        "wander target must be a map neighbor"
    );
    let satiety = satiety_of(&world, eid);
    assert!(
        (satiety - 0.45).abs() < 1e-9,
        "wander must cost exactly metabolism_rate, got {satiety}"
    );
}

#[test]
fn test_ecosystem_wander_deterministic_across_runs() {
    let run_once = || {
        let mut world = ecosystem_world();
        let eid = spawn_wildlife(&mut world, 1, 1, "wander", 0.5, Some("herbivore"));
        EcosystemSystem.run(&mut world);
        cell_of(&world, eid)
    };
    assert_eq!(run_once(), run_once());
}

#[test]
fn test_ecosystem_carnivore_gains_nothing_from_graze() {
    let mut world = ecosystem_world();
    world.time_of_day.day = 0;
    let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("carnivore"));

    EcosystemSystem.run(&mut world);

    assert!((satiety_of(&world, eid) - 0.5).abs() < 1e-9);
}

#[test]
fn test_ecosystem_missing_position_skipped() {
    let mut world = ecosystem_world();
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Wildlife",
            json!({"state": "graze", "satiety": 0.5, "reproduction_cooldown": 0}),
        )
        .unwrap();

    EcosystemSystem.run(&mut world);

    assert!((satiety_of(&world, eid) - 0.5).abs() < 1e-9);
}

#[test]
fn test_ecosystem_missing_species_uses_defaults() {
    let mut world = ecosystem_world();
    world.time_of_day.day = 0;
    let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, None);

    EcosystemSystem.run(&mut world);

    // Default herbivore with default graze_nutrition_rate 0.1.
    assert!((satiety_of(&world, eid) - 0.6).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// M3: flee, rest, reproduction, predation, events
// ---------------------------------------------------------------------------

#[test]
fn test_ecosystem_flee_on_threat_and_increase_distance() {
    let mut world = ecosystem_world();
    let prey = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));
    let threat = spawn_threat(&mut world, 2, 0);

    let before = path_steps_between(&world, &cell_of(&world, prey), &cell_of(&world, threat));

    EcosystemSystem.run(&mut world);

    assert_eq!(state_of(&world, prey), "flee");
    let after = path_steps_between(&world, &cell_of(&world, prey), &cell_of(&world, threat));
    assert!(
        after > before,
        "flee must move away from the threat ({before} -> {after})"
    );
}

#[test]
fn test_ecosystem_flee_recovers_after_three_quiet_ticks() {
    let mut world = ecosystem_world();
    let prey = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));
    let threat = spawn_threat(&mut world, 2, 0);

    EcosystemSystem.run(&mut world);
    assert_eq!(state_of(&world, prey), "flee");

    world.despawn_entity(threat);
    EcosystemSystem.run(&mut world);
    assert_eq!(state_of(&world, prey), "flee");
    EcosystemSystem.run(&mut world);
    assert_eq!(state_of(&world, prey), "flee");
    EcosystemSystem.run(&mut world);
    // Recovery lands back in the graze/wander loop (the graze tick may roll
    // the wander switch, so either loop state counts as recovered).
    assert!(
        ["graze", "wander"].contains(&state_of(&world, prey).as_str()),
        "flee must recover after 3 quiet ticks, got {}",
        state_of(&world, prey)
    );
}

#[test]
fn test_ecosystem_flee_on_noise() {
    use std::collections::HashMap;
    let mut world = ecosystem_world();
    let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));
    world.set_noise_map(HashMap::from([(CellKey::Square { x: 0, y: 0, z: 0 }, 0.9)]));

    EcosystemSystem.run(&mut world);

    assert_eq!(state_of(&world, eid), "flee");
}

#[test]
fn test_ecosystem_diurnal_rests_at_night() {
    let mut world = ecosystem_world();
    world.time_of_day.hour = 0; // night
    world.time_of_day.day = 0; // spring
    let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));

    EcosystemSystem.run(&mut world);

    assert_eq!(state_of(&world, eid), "rest");
    // Stationary with half-rate grazing minus quarter metabolism:
    // 0.5 + 0.2 * 0.5 - 0.05 * 0.25 = 0.5875.
    assert!((satiety_of(&world, eid) - 0.5875).abs() < 1e-9);
    assert_eq!(cell_of(&world, eid), CellKey::Square { x: 0, y: 0, z: 0 });
}

#[test]
fn test_ecosystem_nocturnal_rests_during_day() {
    let mut world = ecosystem_world(); // hour 12: day
    let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));
    set_activity(&mut world, eid, "nocturnal");

    EcosystemSystem.run(&mut world);

    assert_eq!(state_of(&world, eid), "rest");
}

#[test]
fn test_ecosystem_nocturnal_grazes_at_night() {
    let mut world = ecosystem_world();
    world.time_of_day.hour = 0; // night
    world.time_of_day.day = 0;
    let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));
    set_activity(&mut world, eid, "nocturnal");

    EcosystemSystem.run(&mut world);

    // Full spring graze gain, stationary.
    assert!((satiety_of(&world, eid) - 0.7).abs() < 1e-9);
    assert_eq!(cell_of(&world, eid), CellKey::Square { x: 0, y: 0, z: 0 });
}

#[test]
fn test_ecosystem_rest_wakes_into_graze() {
    let mut world = ecosystem_world(); // hour 12: day
    world.time_of_day.day = 0;
    let eid = spawn_wildlife(&mut world, 0, 0, "rest", 0.5, Some("herbivore"));

    EcosystemSystem.run(&mut world);

    // Waking lands back in the graze/wander loop (the graze tick may roll
    // the wander switch, so either loop state counts as awake).
    assert!(
        ["graze", "wander"].contains(&state_of(&world, eid).as_str()),
        "rest must wake when the phase matches, got {}",
        state_of(&world, eid)
    );
}

#[test]
fn test_ecosystem_threat_preempts_rest() {
    let mut world = ecosystem_world();
    world.time_of_day.hour = 0; // night: diurnal would rest
    let prey = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));
    spawn_threat(&mut world, 2, 0);

    EcosystemSystem.run(&mut world);

    assert_eq!(state_of(&world, prey), "flee");
}

#[test]
fn test_ecosystem_reproduce_spawns_child_and_updates_parent() {
    let mut world = ecosystem_world(); // hour 12: day, ambient 15C
    world.time_of_day.day = 0;
    let parent = spawn_wildlife(&mut world, 0, 0, "graze", 0.9, Some("herbivore"));
    let before_count = world.entities.len();

    EcosystemSystem.run(&mut world);

    assert_eq!(world.entities.len(), before_count + 1);
    let child = *world.entities.iter().max().unwrap();
    let child_wildlife = world.get_component(child, "Wildlife").unwrap();
    assert_eq!(
        child_wildlife.get("state").and_then(|v| v.as_str()),
        Some("graze")
    );
    assert_eq!(
        child_wildlife.get("satiety").and_then(|v| v.as_f64()),
        Some(0.5)
    );
    assert_eq!(
        world
            .get_component(child, "Species")
            .unwrap()
            .get("diet")
            .and_then(|v| v.as_str()),
        Some("herbivore")
    );
    // Parent satiety halves, cooldown resets, stored state returns to graze.
    assert!((satiety_of(&world, parent) - 0.45).abs() < 1e-9);
    assert_eq!(
        world
            .get_component(parent, "Wildlife")
            .unwrap()
            .get("reproduction_cooldown")
            .and_then(|v| v.as_u64()),
        Some(10)
    );
    assert_eq!(state_of(&world, parent), "graze");

    world.update_event_buses::<serde_json::Value>();
    let born = world.drain_events::<serde_json::Value>("wildlife_born");
    assert_eq!(born.len(), 1);
    assert_eq!(born[0]["parent"], json!(parent));
    assert_eq!(born[0]["child"], json!(child));
    assert_eq!(born[0]["species_diet"], json!("herbivore"));
    let changed = world.drain_events::<serde_json::Value>("wildlife_state_changed");
    assert!(
        changed.iter().any(|e| e["entity"] == json!(parent)
            && e["old_state"] == json!("graze")
            && e["new_state"] == json!("reproduce")),
        "expected a graze->reproduce transition event, got {changed:?}"
    );
}

#[test]
fn test_ecosystem_reproduce_blocked_outside_temperature_window() {
    for ambient in [-5.0, 50.0] {
        let mut world = ecosystem_world();
        world.time_of_day.day = 0;
        world.temperature.ambient = ambient;
        let parent = spawn_wildlife(&mut world, 0, 0, "graze", 0.9, Some("herbivore"));
        let before_count = world.entities.len();

        EcosystemSystem.run(&mut world);

        assert_eq!(
            world.entities.len(),
            before_count,
            "no birth expected at {ambient}C"
        );
        assert!((satiety_of(&world, parent) - 1.0).abs() < 1e-9);
    }
}

#[test]
fn test_ecosystem_predation_kill_grants_satiety() {
    let mut world = ecosystem_world();
    let predator = spawn_wildlife(&mut world, 0, 0, "wander", 0.5, Some("carnivore"));
    let prey = spawn_wildlife(&mut world, 1, 0, "graze", 0.5, Some("herbivore"));
    world
        .set_component(prey, "Health", json!({"current": 1.0, "max": 10.0}))
        .unwrap();

    EcosystemSystem.run(&mut world);

    assert!(!world.is_entity_alive(prey));
    // Wander metabolism (0.5 - 0.05) plus the kill bonus, clamped.
    assert!((satiety_of(&world, predator) - 0.95).abs() < 1e-9);
}

#[test]
fn test_ecosystem_predation_surviving_prey_grants_no_bonus() {
    let mut world = ecosystem_world();
    let predator = spawn_wildlife(&mut world, 0, 0, "wander", 0.5, Some("carnivore"));
    let prey = spawn_wildlife(&mut world, 1, 0, "graze", 0.5, Some("herbivore"));
    world
        .set_component(prey, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();

    EcosystemSystem.run(&mut world);

    assert!(world.is_entity_alive(prey));
    assert_eq!(
        world
            .get_component(prey, "Health")
            .unwrap()
            .get("current")
            .and_then(|v| v.as_f64()),
        Some(9.0)
    );
    assert!((satiety_of(&world, predator) - 0.45).abs() < 1e-9);
}

#[test]
fn test_ecosystem_unknown_state_treated_as_graze() {
    let mut world = ecosystem_world();
    world.time_of_day.day = 0;
    let eid = spawn_wildlife(&mut world, 0, 0, "graze", 0.5, Some("herbivore"));
    // Schema validation rejects unknown states on the write path, so inject
    // directly to exercise the tolerance arm for legacy/save data.
    world
        .components
        .get_mut("Wildlife")
        .unwrap()
        .insert(eid, json!({"state": "bogus", "satiety": 0.5}));

    EcosystemSystem.run(&mut world);

    assert!((satiety_of(&world, eid) - 0.7).abs() < 1e-9);
}
