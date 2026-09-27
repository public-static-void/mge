//! Standing suite for the Diplomacy AI module (relationships, treaties, war).
//!
//! Created in M1 as a skeleton alongside the F-01 reputation-shape
//! precondition fix. Grows milestone by milestone (M2–M5 extend in place).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::diplomacy::{RelationState, get_relation, get_standing, modify_standing};
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{Map, SquareGridMap};
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

/// Create a world with all schemas and a map, ready for diplomacy testing.
fn setup_world() -> World {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.map = Some(open_plane(10));
    world
}

/// Pair lookups behave identically regardless of argument order.
#[test]
fn pair_lookup_is_order_independent() {
    let mut world = setup_world();
    modify_standing(&mut world, "a", "b", 30).unwrap();
    assert_eq!(
        get_standing(&world, "b", "a"),
        get_standing(&world, "a", "b")
    );
    assert_eq!(get_standing(&world, "b", "a"), 30);
    assert_eq!(
        get_relation(&world, "b", "a"),
        get_relation(&world, "a", "b")
    );
}

/// A faction cannot hold a diplomatic relation with itself.
#[test]
fn self_pair_mutation_is_rejected() {
    let mut world = setup_world();
    assert!(modify_standing(&mut world, "a", "a", 10).is_err());
    world.update_event_buses::<serde_json::Value>();
    let events: Vec<serde_json::Value> = world.drain_events("relation_changed");
    assert!(events.is_empty(), "rejected mutation must emit no event");
}

/// Empty faction ids are invalid on every mutating path.
#[test]
fn empty_faction_id_is_rejected() {
    let mut world = setup_world();
    assert!(modify_standing(&mut world, "", "b", 10).is_err());
    assert!(modify_standing(&mut world, "a", "", 10).is_err());
    world.update_event_buses::<serde_json::Value>();
    let events: Vec<serde_json::Value> = world.drain_events("relation_changed");
    assert!(events.is_empty(), "rejected mutation must emit no event");
}

/// Standing clamps to ±100 on large deltas and on accumulation.
#[test]
fn standing_clamps_to_bounds() {
    let mut world = setup_world();
    modify_standing(&mut world, "a", "b", 200).unwrap();
    assert_eq!(get_standing(&world, "a", "b"), 100);
    modify_standing(&mut world, "c", "d", -200).unwrap();
    assert_eq!(get_standing(&world, "c", "d"), -100);
    modify_standing(&mut world, "e", "f", 90).unwrap();
    modify_standing(&mut world, "e", "f", 30).unwrap();
    assert_eq!(get_standing(&world, "e", "f"), 100);
}

/// Unknown pairs report neutral defaults without creating state.
#[test]
fn unknown_pair_returns_neutral_defaults() {
    let world = setup_world();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::Neutral);
    assert_eq!(get_standing(&world, "a", "b"), 0);
}

/// Each standing mutation emits exactly one relation event.
#[test]
fn standing_mutation_emits_single_event() {
    let mut world = setup_world();
    modify_standing(&mut world, "a", "b", 30).unwrap();
    world.update_event_buses::<serde_json::Value>();
    let events: Vec<serde_json::Value> = world.drain_events("relation_changed");
    assert_eq!(events.len(), 1, "expected exactly one event per mutation");
    let event = &events[0];
    assert_eq!(event.get("a").and_then(|v| v.as_str()), Some("a"));
    assert_eq!(event.get("b").and_then(|v| v.as_str()), Some("b"));
    assert_eq!(event.get("old_standing").and_then(|v| v.as_i64()), Some(0));
    assert_eq!(event.get("new_standing").and_then(|v| v.as_i64()), Some(30));
    assert_eq!(event.get("state").and_then(|v| v.as_str()), Some("Neutral"));
}

/// Regression test for the F-01 precondition: hostile targeting reads the
/// production flat `values` shape written by `modify_reputation`.
#[test]
fn reputation_gated_targeting_uses_production_shape() {
    use engine_core::ecs::system::System;
    use engine_core::faction::modify_reputation;
    use engine_core::systems::enemy_behavior::EnemyBehaviorSystem;
    use serde_json::json;

    let mut world = setup_world();

    let enemy = world.spawn_entity();
    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "idle",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null
            }),
        )
        .unwrap();
    world
        .set_component(
            enemy,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(enemy, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();
    world
        .set_component(enemy, "Sight", json!({"range": 10}))
        .unwrap();
    world
        .set_component(
            enemy,
            "Faction",
            json!({"faction_id": "raiders", "role": "enemy"}),
        )
        .unwrap();
    world
        .set_component(enemy, "Type", json!({"kind": "enemy"}))
        .unwrap();

    let player = world.spawn_entity();
    world
        .set_component(
            player,
            "Position",
            json!({"pos": {"Square": {"x": 3, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(player, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();
    world
        .set_component(player, "Type", json!({"kind": "villager"}))
        .unwrap();
    world
        .set_component(
            player,
            "Faction",
            json!({"faction_id": "players", "role": "member"}),
        )
        .unwrap();
    modify_reputation(&mut world, player, "raiders", -50).unwrap();

    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    visible.insert(CellKey::Square { x: 3, y: 0, z: 0 });
    world.set_visible_cells(enemy, visible);

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = world
        .get_component(enemy, "EnemyAI")
        .and_then(|ai| ai.get("state"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .unwrap();
    assert_eq!(
        state, "chase",
        "Enemy should detect and chase hostile faction member written via production shape"
    );
}
