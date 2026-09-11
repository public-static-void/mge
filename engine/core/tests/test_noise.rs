//! Integration tests for the NoiseSystem and noise-based enemy detection.
//!
//! Tests noise emission + BFS propagation with linear falloff, wall blocking,
//! multi-source max aggregation, hearing detection, alert_level escalation,
//! stealth modifier, and determinism (AC031, AC040).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::enemy_behavior::EnemyBehaviorSystem;
use engine_core::systems::noise::NoiseSystem;
use serde_json::json;

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

/// Create a world with all schemas and an open plane map, ready for noise testing.
fn setup_noise_world() -> World {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.map = Some(open_plane(10));
    world
}

/// Spawn an entity with a NoiseEmitter component at (x, y, 0).
fn spawn_emitter(world: &mut World, x: i32, y: i32, intensity: f64, radius: u32) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "NoiseEmitter",
            json!({
                "intensity": intensity,
                "radius": radius,
                "active": true,
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
    eid
}

/// Spawn an idle enemy with EnemyAI + Hearing at (x, y, 0).
fn spawn_hearing_enemy(world: &mut World, x: i32, y: i32) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "EnemyAI",
            json!({
                "state": "idle",
                "alert_level": 0,
                "detection_range": 8,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": null,
                "target_entity": null
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
        .set_component(eid, "Hearing", json!({"range": 5, "sensitivity": 1.0}))
        .unwrap();
    world
        .set_component(eid, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();
    eid
}

/// Get the AI state of an entity.
fn get_ai_state(world: &World, eid: u32) -> Option<String> {
    world
        .get_component(eid, "EnemyAI")
        .and_then(|ai| ai.get("state"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

/// Get the alert_level of an entity.
fn get_alert_level(world: &World, eid: u32) -> f64 {
    world
        .get_component(eid, "EnemyAI")
        .and_then(|ai| ai.get("alert_level"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn noise_system_name() {
    let system = NoiseSystem;
    assert_eq!(system.name(), "NoiseSystem");
}

#[test]
fn noise_system_dependencies() {
    let system = NoiseSystem;
    assert_eq!(system.dependencies(), &["FovUpdateSystem"]);
}

#[test]
fn test_noise_emission_propagation() {
    let mut world = setup_noise_world();
    spawn_emitter(&mut world, 0, 0, 1.0, 5);

    let mut system = NoiseSystem;
    system.run(&mut world);

    // Origin cell receives full intensity (R006)
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 0, y: 0, z: 0 }),
        Some(1.0),
        "Origin should receive full intensity"
    );

    // Linear falloff: intensity * (radius - distance) / radius
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 1, y: 0, z: 0 }),
        Some(0.8),
        "Distance 1 should receive 0.8"
    );
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 2, y: 0, z: 0 }),
        Some(0.6),
        "Distance 2 should receive 0.6"
    );
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 3, y: 0, z: 0 }),
        Some(0.4),
        "Distance 3 should receive 0.4"
    );
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 4, y: 0, z: 0 }),
        Some(0.2),
        "Distance 4 should receive 0.2"
    );

    // Beyond radius: no noise propagated
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 6, y: 0, z: 0 }),
        None,
        "Cells beyond radius should have no noise"
    );
}

#[test]
fn test_noise_wall_blocking() {
    // Narrow 1-cell-wide corridor along the x-axis — the only path is through
    // the wall, so an opaque cell must fully block propagation (R007).
    let mut grid = SquareGridMap::new();
    for x in 0..=4 {
        grid.add_cell(x, 0, 0);
    }
    for x in 0..4 {
        grid.add_neighbor((x, 0, 0), (x + 1, 0, 0));
    }
    let mut map = Map::new(Box::new(grid));
    map.set_cell_metadata(
        &CellKey::Square { x: 1, y: 0, z: 0 },
        json!({"transparent": false}),
    );

    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.map = Some(map);
    spawn_emitter(&mut world, 0, 0, 1.0, 5);

    let mut system = NoiseSystem;
    system.run(&mut world);

    // The wall cell itself receives noise
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 1, y: 0, z: 0 }),
        Some(0.8),
        "Wall cell should receive noise"
    );

    // Cells beyond the wall are unreachable — silent
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 2, y: 0, z: 0 }),
        None,
        "Cell behind wall should be silent"
    );
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 3, y: 0, z: 0 }),
        None,
        "Cell two steps behind wall should be silent"
    );
}

#[test]
fn test_noise_multi_source_aggregation() {
    let mut world = setup_noise_world();
    // Emitter A: intensity 1.0 at (0, 0); Emitter B: intensity 0.5 at (0, 2)
    spawn_emitter(&mut world, 0, 0, 1.0, 5);
    spawn_emitter(&mut world, 0, 2, 0.5, 5);

    let mut system = NoiseSystem;
    system.run(&mut world);

    // At (0, 1): A contributes 0.8, B contributes 0.4 → max = 0.8 (R008)
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 0, y: 1, z: 0 }),
        Some(0.8),
        "Max aggregation should pick the louder source"
    );

    // At (0, 2): A contributes 0.6, B contributes 0.5 → max = 0.6
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 0, y: 2, z: 0 }),
        Some(0.6),
        "Max aggregation at the second emitter's cell"
    );
}

#[test]
fn test_hearing_detection_triggers_alert() {
    let mut world = setup_noise_world();
    // Enemy at (1, 0) with Hearing; emitter at (0, 0) intensity 0.5
    // → noise at (1, 0) = 0.5 * (5-1)/5 = 0.4
    let enemy = spawn_hearing_enemy(&mut world, 1, 0);
    spawn_emitter(&mut world, 0, 0, 0.5, 5);

    let mut noise_system = NoiseSystem;
    noise_system.run(&mut world);
    let mut behavior = EnemyBehaviorSystem;
    behavior.run(&mut world);

    // alert_level incremented by noise × sensitivity = 0.4 (AC015)
    assert_eq!(
        get_alert_level(&world, enemy),
        0.4,
        "alert_level should increment by noise × sensitivity"
    );
    // 0.4 > noise_detection_threshold (0.3) → investigate (AC017)
    assert_eq!(
        get_ai_state(&world, enemy).unwrap(),
        "investigate",
        "Enemy should investigate after hearing noise above threshold"
    );
}

#[test]
fn test_alert_level_escalation_to_investigate_and_chase() {
    // Scenario 1: moderate noise (0.4) → investigate
    let mut world = setup_noise_world();
    let enemy = spawn_hearing_enemy(&mut world, 1, 0);
    spawn_emitter(&mut world, 0, 0, 0.5, 5);
    let mut noise_system = NoiseSystem;
    noise_system.run(&mut world);
    let mut behavior = EnemyBehaviorSystem;
    behavior.run(&mut world);
    assert_eq!(
        get_ai_state(&world, enemy).unwrap(),
        "investigate",
        "alert_level 0.4 should transition to investigate"
    );

    // Scenario 2: loud noise (0.8) → chase (AC018)
    let mut world2 = setup_noise_world();
    let enemy2 = spawn_hearing_enemy(&mut world2, 1, 0);
    spawn_emitter(&mut world2, 0, 0, 1.0, 5);
    let mut noise_system2 = NoiseSystem;
    noise_system2.run(&mut world2);
    let mut behavior2 = EnemyBehaviorSystem;
    behavior2.run(&mut world2);
    assert_eq!(
        get_ai_state(&world2, enemy2).unwrap(),
        "chase",
        "alert_level 0.8 should transition to chase"
    );
}

#[test]
fn test_noise_determinism() {
    // Same scenario run twice must produce identical noise maps (AC040, NFR002)
    let mut world1 = setup_noise_world();
    spawn_emitter(&mut world1, 0, 0, 1.0, 5);
    spawn_emitter(&mut world1, 3, 2, 0.7, 4);
    let mut system1 = NoiseSystem;
    system1.run(&mut world1);

    let mut world2 = setup_noise_world();
    spawn_emitter(&mut world2, 0, 0, 1.0, 5);
    spawn_emitter(&mut world2, 3, 2, 0.7, 4);
    let mut system2 = NoiseSystem;
    system2.run(&mut world2);

    assert_eq!(
        world1.noise_map, world2.noise_map,
        "Identical scenarios must produce identical noise maps"
    );
}

#[test]
fn test_stealth_reduces_noise() {
    let mut world = setup_noise_world();
    let emitter = spawn_emitter(&mut world, 0, 0, 1.0, 5);
    // Stealth halves the effective intensity (R004)
    world
        .set_component(emitter, "Stealth", json!({"noise_modifier": 0.5}))
        .unwrap();

    let mut system = NoiseSystem;
    system.run(&mut world);

    // Effective intensity = 1.0 * 0.5 = 0.5
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 0, y: 0, z: 0 }),
        Some(0.5),
        "Stealth should halve origin noise"
    );
    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 1, y: 0, z: 0 }),
        Some(0.4),
        "Stealth should halve propagated noise"
    );
}

#[test]
fn test_inactive_emitter_is_skipped() {
    let mut world = setup_noise_world();
    let emitter = spawn_emitter(&mut world, 0, 0, 1.0, 5);
    // Deactivate the emitter (R010)
    world
        .set_component(
            emitter,
            "NoiseEmitter",
            json!({
                "intensity": 1.0,
                "radius": 5,
                "active": false,
            }),
        )
        .unwrap();

    let mut system = NoiseSystem;
    system.run(&mut world);

    assert_eq!(
        world.get_noise_at(&CellKey::Square { x: 0, y: 0, z: 0 }),
        None,
        "Inactive emitters should not propagate noise"
    );
}
