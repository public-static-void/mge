//! Integration tests for the EnemyBehaviorSystem.
//!
//! Tests the deterministic finite-state-machine: idle/patrol/chase/attack/flee states,
//! target selection, A* pathfinding movement, and deterministic RNG.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::enemy_behavior::EnemyBehaviorSystem;
use serde_json::json;
use std::collections::HashSet;

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

/// Create a world with all schemas and a map, ready for enemy AI testing.
fn setup_enemy_ai_world() -> World {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.map = Some(open_plane(10));
    world
}

/// Spawn an enemy entity with all required components.
fn spawn_enemy(
    world: &mut World,
    x: i32,
    y: i32,
    state: &str,
    detection_range: u64,
    target_faction: Option<&str>,
) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "EnemyAI",
            json!({
                "state": state,
                "alert_level": 0,
                "detection_range": detection_range,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": target_faction,
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
        .set_component(eid, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();
    world
        .set_component(eid, "Sight", json!({"range": 10}))
        .unwrap();
    world
        .set_component(
            eid,
            "Faction",
            json!({"faction_id": "enemies", "role": "enemy"}),
        )
        .unwrap();
    world
        .set_component(eid, "Type", json!({"kind": "enemy"}))
        .unwrap();
    eid
}

/// Spawn a player entity (target for enemies).
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

/// Get the AI state of an entity.
fn get_ai_state(world: &World, eid: u32) -> Option<String> {
    world
        .get_component(eid, "EnemyAI")
        .and_then(|ai| ai.get("state"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

/// Get the position of an entity.
fn get_position(world: &World, eid: u32) -> Option<(i32, i32, i32)> {
    world.get_component(eid, "Position").and_then(|pos| {
        let square = pos.get("pos")?.get("Square")?;
        Some((
            square.get("x")?.as_i64()? as i32,
            square.get("y")?.as_i64()? as i32,
            square.get("z")?.as_i64()? as i32,
        ))
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn enemy_system_name() {
    let system = EnemyBehaviorSystem;
    assert_eq!(system.name(), "EnemyBehaviorSystem");
}

#[test]
fn enemy_system_dependencies() {
    let system = EnemyBehaviorSystem;
    assert_eq!(system.dependencies(), &["FovUpdateSystem", "NoiseSystem"]);
}

#[test]
fn idle_to_chase_on_hostile_detection() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "idle", 10, Some("players"));
    let _player = spawn_player(&mut world, 3, 0);

    // Set up visible cells so the enemy can "see" the player
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 3, y: 0, z: 0 });
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    world.set_visible_cells(enemy, visible);

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "chase",
        "Enemy should transition to chase when hostile detected"
    );
}

#[test]
fn idle_to_patrol_when_route_exists() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "idle", 5, None);

    // Set patrol route
    world
        .set_component(
            enemy,
            "PatrolRoute",
            json!({
                "waypoints": [
                    {"Square": {"x": 3, "y": 0, "z": 0}},
                    {"Square": {"x": 3, "y": 3, "z": 0}}
                ],
                "current_index": 0,
                "loop": true,
                "wait_ticks": 0
            }),
        )
        .unwrap();

    // No visible hostile targets — idle should transition to patrol
    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "patrol",
        "Enemy should transition to patrol when route exists"
    );
}

#[test]
fn patrol_moves_toward_waypoint() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "patrol", 5, None);

    // Set patrol route with one waypoint at (3, 0, 0)
    world
        .set_component(
            enemy,
            "PatrolRoute",
            json!({
                "waypoints": [
                    {"Square": {"x": 3, "y": 0, "z": 0}}
                ],
                "current_index": 0,
                "loop": true,
                "wait_ticks": 0
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let pos = get_position(&world, enemy).unwrap();
    // Should have moved one step toward (3, 0, 0)
    assert!(
        pos.0 > 0 || pos.1 > 0,
        "Enemy should have moved toward waypoint: {:?}",
        pos
    );
}

#[test]
fn patrol_advances_index_on_arrival() {
    let mut world = setup_enemy_ai_world();
    // Place enemy at the first waypoint
    let enemy = spawn_enemy(&mut world, 3, 0, "patrol", 5, None);

    // Route: first waypoint is (3, 0), second is (3, 3)
    world
        .set_component(
            enemy,
            "PatrolRoute",
            json!({
                "waypoints": [
                    {"Square": {"x": 3, "y": 0, "z": 0}},
                    {"Square": {"x": 3, "y": 3, "z": 0}}
                ],
                "current_index": 0,
                "loop": true,
                "wait_ticks": 0
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    // Should have advanced to next waypoint
    let route = world.get_component(enemy, "PatrolRoute").unwrap();
    let idx = route.get("current_index").and_then(|v| v.as_u64()).unwrap();
    assert_eq!(idx, 1, "current_index should advance to 1");
}

#[test]
fn patrol_loops_when_complete() {
    let mut world = setup_enemy_ai_world();
    // Place enemy at the second (last) waypoint
    let enemy = spawn_enemy(&mut world, 3, 3, "patrol", 5, None);

    // Route: two waypoints, enemy is at index 1 (the last)
    world
        .set_component(
            enemy,
            "PatrolRoute",
            json!({
                "waypoints": [
                    {"Square": {"x": 3, "y": 0, "z": 0}},
                    {"Square": {"x": 3, "y": 3, "z": 0}}
                ],
                "current_index": 1,
                "loop": true,
                "wait_ticks": 0
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    // Should loop back to index 0
    let route = world.get_component(enemy, "PatrolRoute").unwrap();
    let idx = route.get("current_index").and_then(|v| v.as_u64()).unwrap();
    assert_eq!(idx, 0, "current_index should reset to 0 when looping");
}

#[test]
fn patrol_to_idle_when_no_loop() {
    let mut world = setup_enemy_ai_world();
    // Place enemy at the last waypoint
    let enemy = spawn_enemy(&mut world, 3, 3, "patrol", 5, None);

    world
        .set_component(
            enemy,
            "PatrolRoute",
            json!({
                "waypoints": [
                    {"Square": {"x": 3, "y": 0, "z": 0}},
                    {"Square": {"x": 3, "y": 3, "z": 0}}
                ],
                "current_index": 1,
                "loop": false,
                "wait_ticks": 0
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "idle",
        "Should return to idle when patrol completes without loop"
    );
}

#[test]
fn chase_to_attack_when_in_range() {
    let mut world = setup_enemy_ai_world();
    // Enemy at (0, 0), player at (0, 1) — within attack_range=1
    let enemy = spawn_enemy(&mut world, 0, 0, "chase", 10, Some("players"));
    let player = spawn_player(&mut world, 0, 1);

    // Player must be visible for the system to consider them as target
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    visible.insert(CellKey::Square { x: 0, y: 1, z: 0 });
    world.set_visible_cells(enemy, visible);

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "chase",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": player,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 0
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "attack",
        "Enemy should transition to attack when target is in range"
    );
}

#[test]
fn attack_deals_damage() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "attack", 10, Some("players"));
    let player = spawn_player(&mut world, 0, 1);

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "attack",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": player,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 0
            }),
        )
        .unwrap();

    let health_before = world
        .get_component(player, "Health")
        .and_then(|h| h.get("current"))
        .and_then(|v| v.as_f64())
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let health_after = world
        .get_component(player, "Health")
        .and_then(|h| h.get("current"))
        .and_then(|v| v.as_f64())
        .unwrap();

    assert!(
        health_after < health_before,
        "Player health should decrease after attack: {} -> {}",
        health_before,
        health_after
    );
}

#[test]
fn attack_to_idle_when_target_dies() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "attack", 10, Some("players"));
    let player = spawn_player(&mut world, 0, 1);

    // Kill the player
    world
        .set_component(player, "Health", json!({"current": 0.0, "max": 10.0}))
        .unwrap();

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "attack",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": player,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 0
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "idle",
        "Enemy should return to idle when target dies"
    );
}

#[test]
fn chase_to_flee_at_low_health() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "chase", 10, Some("players"));
    let player = spawn_player(&mut world, 3, 0);

    // Set enemy health to low (below flee_threshold of 0.25)
    world
        .set_component(enemy, "Health", json!({"current": 2.0, "max": 10.0}))
        .unwrap();

    // Set visible cells
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    visible.insert(CellKey::Square { x: 3, y: 0, z: 0 });
    world.set_visible_cells(enemy, visible);

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "chase",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": player,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 0
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(state, "flee", "Enemy should flee when health is low");
}

#[test]
fn flee_to_idle_after_recovery() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "flee", 10, Some("players"));
    let _player = spawn_player(&mut world, 3, 0);

    // Set enemy health above flee_threshold
    world
        .set_component(enemy, "Health", json!({"current": 5.0, "max": 10.0}))
        .unwrap();

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "flee",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": null,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 0,
                "flee_ticks": 2
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "idle",
        "Enemy should return to idle after 3 flee ticks with recovered health"
    );
}

#[test]
fn chase_returns_home_after_losing_target() {
    let mut world = setup_enemy_ai_world();
    // Place enemy already at home position, with dead/nonexistent target
    let enemy = spawn_enemy(&mut world, 0, 0, "chase", 10, Some("players"));
    let _player = spawn_player(&mut world, 5, 0);

    // Set visible cells that DON'T include the player (target is lost)
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    world.set_visible_cells(enemy, visible);

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "chase",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": 999,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 2
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    // Enemy at home position with dead target → should return to idle
    assert_eq!(
        state, "idle",
        "Enemy should return to idle when at home and target is dead"
    );
}

#[test]
fn chase_navigates_home_when_target_lost() {
    let mut world = setup_enemy_ai_world();
    // Enemy is NOT at home — should start navigating home
    let enemy = spawn_enemy(&mut world, 5, 5, "chase", 10, Some("players"));
    let _player = spawn_player(&mut world, 10, 10);

    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 5, y: 5, z: 0 });
    world.set_visible_cells(enemy, visible);

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "chase",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": 999,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 2
            }),
        )
        .unwrap();

    let pos_before = get_position(&world, enemy).unwrap();
    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);
    let pos_after = get_position(&world, enemy).unwrap();

    // Should have moved toward home (0, 0, 0)
    assert_ne!(
        pos_before, pos_after,
        "Enemy should move toward home when target is lost"
    );
    // State should still be chase (moving, not arrived yet)
    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(state, "chase", "State remains chase while navigating home");
}

#[test]
fn target_selection_by_faction() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "idle", 10, Some("players"));
    let player = spawn_player(&mut world, 3, 0);

    // Add Faction and Reputation to the player
    world
        .set_component(
            player,
            "Faction",
            json!({"faction_id": "players", "role": "member"}),
        )
        .unwrap();
    world
        .set_component(
            player,
            "Reputation",
            json!({"relations": {"enemies": {"score": -50}}}),
        )
        .unwrap();

    // Set visible cells
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    visible.insert(CellKey::Square { x: 3, y: 0, z: 0 });
    world.set_visible_cells(enemy, visible);

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "chase",
        "Enemy should detect and chase hostile faction member"
    );
}

#[test]
fn determinism_same_input_same_output() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "idle", 10, Some("players"));
    let _player = spawn_player(&mut world, 3, 0);

    // Set visible cells
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    visible.insert(CellKey::Square { x: 3, y: 0, z: 0 });
    world.set_visible_cells(enemy, visible);

    // Run 1
    let mut sys1 = EnemyBehaviorSystem;
    sys1.run(&mut world);
    let state1 = get_ai_state(&world, enemy).unwrap();
    let pos1 = get_position(&world, enemy);

    // Reset for run 2
    let mut world2 = setup_enemy_ai_world();
    let enemy2 = spawn_enemy(&mut world2, 0, 0, "idle", 10, Some("players"));
    let _player2 = spawn_player(&mut world2, 3, 0);

    let mut visible2 = HashSet::new();
    visible2.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    visible2.insert(CellKey::Square { x: 3, y: 0, z: 0 });
    world2.set_visible_cells(enemy2, visible2);

    let mut sys2 = EnemyBehaviorSystem;
    sys2.run(&mut world2);
    let state2 = get_ai_state(&world2, enemy2).unwrap();
    let pos2 = get_position(&world2, enemy2);

    assert_eq!(state1, state2, "Determinism: same state after same inputs");
    assert_eq!(pos1, pos2, "Determinism: same position after same inputs");
}

#[test]
fn patrol_to_chase_when_hostile_detected() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "patrol", 10, Some("players"));
    let player = spawn_player(&mut world, 2, 0);

    // Set patrol route
    world
        .set_component(
            enemy,
            "PatrolRoute",
            json!({
                "waypoints": [
                    {"Square": {"x": 5, "y": 0, "z": 0}}
                ],
                "current_index": 0,
                "loop": true,
                "wait_ticks": 0
            }),
        )
        .unwrap();

    // Set visible cells including the player
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    visible.insert(CellKey::Square { x: 2, y: 0, z: 0 });
    world.set_visible_cells(enemy, visible);

    // Add Faction and Reputation to the player for faction-based targeting
    world
        .set_component(
            player,
            "Faction",
            json!({"faction_id": "players", "role": "member"}),
        )
        .unwrap();
    world
        .set_component(
            player,
            "Reputation",
            json!({"relations": {"enemies": {"score": -50}}}),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "chase",
        "Patrol should transition to chase when hostile detected"
    );
}

#[test]
fn idle_stays_idle_when_no_target_no_route() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "idle", 10, None);

    // No visible hostile targets, no patrol route
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    world.set_visible_cells(enemy, visible);

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "idle",
        "Enemy should stay idle with no targets and no route"
    );
}

#[test]
fn attack_to_chase_when_target_moves_out_of_range() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "attack", 10, Some("players"));
    let player = spawn_player(&mut world, 5, 0); // Far away

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "attack",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": player,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 0
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "chase",
        "Attack should transition to chase when target moves out of range"
    );
}

#[test]
fn flee_moves_away_from_target() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 3, 0, "flee", 10, Some("players"));
    let _player = spawn_player(&mut world, 0, 0);

    // Set enemy health above flee threshold so it transitions to idle after 3 ticks
    world
        .set_component(enemy, "Health", json!({"current": 5.0, "max": 10.0}))
        .unwrap();

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "flee",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": 100,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 0,
                "flee_ticks": 0
            }),
        )
        .unwrap();

    let pos_before = get_position(&world, enemy).unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let pos_after = get_position(&world, enemy).unwrap();
    // Enemy should have moved (either away from target or random walk)
    assert_ne!(pos_before, pos_after, "Enemy should move during flee");
}

#[test]
fn attack_flee_at_low_health() {
    let mut world = setup_enemy_ai_world();
    let enemy = spawn_enemy(&mut world, 0, 0, "attack", 10, Some("players"));
    let player = spawn_player(&mut world, 0, 1);

    // Set enemy health to low (below flee_threshold of 0.25)
    world
        .set_component(enemy, "Health", json!({"current": 1.0, "max": 10.0}))
        .unwrap();

    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "attack",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null,
                "current_target": player,
                "home_position": {"Square": {"x": 0, "y": 0, "z": 0}},
                "lost_target_ticks": 0
            }),
        )
        .unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = get_ai_state(&world, enemy).unwrap();
    assert_eq!(
        state, "flee",
        "Enemy should flee when health is low even during attack"
    );
}
