//! Integration tests for task decoupling: builtin job handlers route completion
//! through the per-job `required_progress` field, and enemy behavior numerics
//! are component-read params with named fallbacks.
//!
//! Assertions compare runtime behavior against the named consts, never const
//! against literal, following the `test_named_defaults.rs` convention.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::enemy_behavior::EnemyBehaviorSystem;
use engine_core::systems::enemy_behavior::enemy_defaults;
use engine_core::systems::job::builtin_handlers::register_builtin_job_handlers;
use engine_core::systems::job::types::job_type::{DEFAULT_REQUIRED_PROGRESS, JobTypeRegistry};
use serde_json::json;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

/// Build an open plane map with 8-directional adjacency.
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

fn setup_ai_world() -> World {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.map = Some(open_plane(10));
    world
}

/// Spawn an enemy with an `EnemyAI` component built from the given extra
/// fields merged over the minimal idle baseline.
fn spawn_enemy_with(world: &mut World, x: i32, y: i32, ai_extra: serde_json::Value) -> u32 {
    let eid = world.spawn_entity();
    let mut ai = json!({
        "state": "idle",
        "alert_level": 0,
        "target_faction": null,
        "target_entity": null,
    });
    for (k, v) in ai_extra.as_object().cloned().unwrap_or_default() {
        ai[k] = v;
    }
    world.set_component(eid, "EnemyAI", ai).unwrap();
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
    world
        .set_component(eid, "Type", json!({"kind": "enemy"}))
        .unwrap();
    eid
}

fn spawn_player(world: &mut World, x: i32, y: i32) -> u32 {
    let eid = world.spawn_entity();
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
    world
        .set_component(eid, "Type", json!({"kind": "player"}))
        .unwrap();
    eid
}

fn ai_state(world: &World, eid: u32) -> String {
    world
        .get_component(eid, "EnemyAI")
        .and_then(|ai| ai.get("state"))
        .and_then(|s| s.as_str())
        .unwrap_or("<missing>")
        .to_string()
}

fn visible_pair(enemy: (i32, i32), other: (i32, i32)) -> HashSet<CellKey> {
    HashSet::from([
        CellKey::Square {
            x: enemy.0,
            y: enemy.1,
            z: 0,
        },
        CellKey::Square {
            x: other.0,
            y: other.1,
            z: 0,
        },
    ])
}

// ---------------------------------------------------------------------------
// Builtin handler routing through required_progress
// ---------------------------------------------------------------------------

/// Register the builtin handler for a single `haul` job type in a temp dir.
fn world_with_builtin_haul() -> (World, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("temp dir for job definitions");
    std::fs::write(
        temp_dir.path().join("haul.json"),
        r#"{"name":"haul","type":"haul"}"#,
    )
    .unwrap();
    let mut world = make_test_world();
    let mut registry = JobTypeRegistry::default();
    registry.register_native("haul", |_, _, _, job| job.clone());
    register_builtin_job_handlers(&mut world, &registry, temp_dir.path());
    (world, temp_dir)
}

/// Drive the registered builtin handler `n` times, writing each result back
/// to the job component the way successive ticks would.
fn drive_handler(world: &mut World, job_id: u32, n: usize) -> serde_json::Value {
    let handler = world
        .job_handler_registry
        .lock()
        .unwrap()
        .get("haul")
        .cloned()
        .expect("builtin handler registered for haul");
    let mut current = world.get_component(job_id, "Job").unwrap().clone();
    for _ in 0..n {
        current = handler(world, 0, job_id, &current);
        world.set_component(job_id, "Job", current.clone()).unwrap();
    }
    current
}

#[test]
fn builtin_handler_completes_at_job_required_progress() {
    let (mut world, _dir) = world_with_builtin_haul();
    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "haul",
                "state": "pending",
                "progress": 0.0,
                "required_progress": 5.0,
                "category": "testing",
            }),
        )
        .unwrap();

    let after_three = drive_handler(&mut world, job_id, 3);
    assert_eq!(
        after_three.get("progress").and_then(|v| v.as_f64()),
        Some(3.0)
    );
    assert_eq!(
        after_three.get("state").and_then(|v| v.as_str()),
        Some("in_progress"),
        "a job requiring 5.0 must not complete at the old hard-coded threshold"
    );

    let after_five = drive_handler(&mut world, job_id, 2);
    assert_eq!(
        after_five.get("progress").and_then(|v| v.as_f64()),
        Some(5.0)
    );
    assert_eq!(
        after_five.get("state").and_then(|v| v.as_str()),
        Some("complete")
    );
}

#[test]
fn builtin_handler_falls_back_to_named_required_progress() {
    let (mut world, _dir) = world_with_builtin_haul();
    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "haul",
                "state": "pending",
                "progress": 0.0,
                "category": "testing",
            }),
        )
        .unwrap();

    let ticks = DEFAULT_REQUIRED_PROGRESS as usize;
    let done = drive_handler(&mut world, job_id, ticks);
    assert_eq!(
        done.get("progress").and_then(|v| v.as_f64()),
        Some(DEFAULT_REQUIRED_PROGRESS),
        "a job without required_progress completes exactly at the named default"
    );
    assert_eq!(done.get("state").and_then(|v| v.as_str()), Some("complete"));
}

// ---------------------------------------------------------------------------
// Enemy behavior named fallbacks
// ---------------------------------------------------------------------------

#[test]
fn enemy_detection_falls_back_to_named_default() {
    let range = enemy_defaults::DEFAULT_DETECTION_RANGE as i32;

    // At exactly the default range the fallback still spots the player.
    let mut world = setup_ai_world();
    let enemy = spawn_enemy_with(&mut world, 0, 0, json!({}));
    let _player = spawn_player(&mut world, range, 0);
    world.set_visible_cells(enemy, visible_pair((0, 0), (range, 0)));
    EnemyBehaviorSystem.run(&mut world);
    assert_eq!(ai_state(&world, enemy), "chase");

    // One cell beyond the default range the fallback does not spot them.
    let mut far_world = setup_ai_world();
    let far_enemy = spawn_enemy_with(&mut far_world, 0, 0, json!({}));
    let _far_player = spawn_player(&mut far_world, range + 1, 0);
    far_world.set_visible_cells(far_enemy, visible_pair((0, 0), (range + 1, 0)));
    EnemyBehaviorSystem.run(&mut far_world);
    assert_eq!(ai_state(&far_world, far_enemy), "idle");
}

#[test]
fn enemy_flee_falls_back_to_named_default() {
    let threshold = enemy_defaults::DEFAULT_FLEE_THRESHOLD;
    let max = 4.0;
    let weak = threshold * max;

    let mut world = setup_ai_world();
    let player = spawn_player(&mut world, 2, 0);
    let enemy = spawn_enemy_with(
        &mut world,
        0,
        0,
        json!({"state": "chase", "current_target": player}),
    );
    // Health fraction sits exactly on the named threshold with no
    // flee_threshold key on the component.
    world
        .set_component(enemy, "Health", json!({"current": weak, "max": max}))
        .unwrap();
    world.set_visible_cells(enemy, visible_pair((0, 0), (2, 0)));
    EnemyBehaviorSystem.run(&mut world);
    assert_eq!(
        ai_state(&world, enemy),
        "flee",
        "health fraction at the named threshold flees"
    );

    // Full health keeps chasing: distance 2 also pins the default attack
    // range of 1 from above (no attack transition).
    let mut healthy_world = setup_ai_world();
    let healthy_player = spawn_player(&mut healthy_world, 2, 0);
    let healthy = spawn_enemy_with(
        &mut healthy_world,
        0,
        0,
        json!({"state": "chase", "current_target": healthy_player}),
    );
    healthy_world.set_visible_cells(healthy, visible_pair((0, 0), (2, 0)));
    EnemyBehaviorSystem.run(&mut healthy_world);
    assert_eq!(ai_state(&healthy_world, healthy), "chase");
}

#[test]
fn enemy_attack_damage_matches_named_default() {
    let mut world = setup_ai_world();
    let player = spawn_player(&mut world, 1, 0);
    let enemy = spawn_enemy_with(
        &mut world,
        0,
        0,
        json!({"state": "attack", "current_target": player}),
    );
    // Adjacent target with no attack_range key: the default range of 1 keeps
    // the attack in range and the named damage is applied.
    EnemyBehaviorSystem.run(&mut world);
    assert_eq!(
        ai_state(&world, enemy),
        "attack",
        "attacker stays engaged while the target is in default range"
    );
    let current = world
        .get_component(player, "Health")
        .and_then(|h| h.get("current"))
        .and_then(|v| v.as_f64())
        .unwrap();
    assert!(
        (current - (10.0 - f64::from(enemy_defaults::DEFAULT_ATTACK_DAMAGE))).abs() < f64::EPSILON,
        "attack deals the named default damage, got {current}"
    );
}

#[test]
fn enemy_lost_target_gives_up_at_named_default() {
    // Target stays alive but invisible; the chase gives up after exactly the
    // named number of lost ticks.
    let mut world = setup_ai_world();
    let player = spawn_player(&mut world, 5, 0);
    let enemy = spawn_enemy_with(
        &mut world,
        0,
        0,
        json!({"state": "chase", "current_target": player}),
    );
    // Visible set omits the target cell, so every tick counts as lost.
    world.set_visible_cells(enemy, HashSet::from([CellKey::Square { x: 0, y: 0, z: 0 }]));

    let mut system = EnemyBehaviorSystem;
    for _ in 0..(enemy_defaults::DEFAULT_MAX_LOST_TICKS - 1) {
        system.run(&mut world);
        assert_eq!(ai_state(&world, enemy), "chase");
    }
    system.run(&mut world);
    assert_eq!(
        ai_state(&world, enemy),
        "idle",
        "chase gives up exactly at the named lost-tick limit"
    );
}

#[test]
fn enemy_flee_recovers_at_named_default() {
    // One tick short of the named recovery limit the enemy still flees; the
    // next tick it recovers to idle at full health.
    let mut world = setup_ai_world();
    let enemy = spawn_enemy_with(
        &mut world,
        0,
        0,
        json!({
            "state": "flee",
            "flee_ticks": enemy_defaults::DEFAULT_FLEE_RECOVERY_TICKS - 2,
        }),
    );

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);
    assert_eq!(ai_state(&world, enemy), "flee");
    system.run(&mut world);
    assert_eq!(
        ai_state(&world, enemy),
        "idle",
        "flee recovers exactly at the named recovery limit"
    );
}
