//! Standing suite for the Diplomacy AI module (relationships, treaties, war).
//!
//! Created in M1 as a skeleton alongside the F-01 reputation-shape
//! precondition fix. Grows milestone by milestone (M2–M5 extend in place).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

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

/// Placeholder for future diplomacy behavior tests (M2 onward).
#[test]
#[ignore = "skeleton placeholder — no diplomacy behavior implemented yet (M1)"]
fn diplomacy_placeholder() {
    let world = setup_world();
    let _ = world;
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
