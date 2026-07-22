//! Consolidated visibility tests: fog of war + field of view.

use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::fov::{BfsFovAlgorithm, FovAlgorithm, compute_fov};
use engine_core::map::{HexGridMap, Map, MapTopology, SquareGridMap};
use engine_core::systems::fog::FogUpdateSystem;
use engine_core::systems::fov::FovUpdateSystem;
use serde_json::json;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

// --- Shared helpers ---

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

fn setup_world_with_schema() -> World {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        reg.register_external_schema(ComponentSchema {
            name: "Sight".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/sight.json")).unwrap(),
            modes: vec![
                "colony".to_string(),
                "roguelike".to_string(),
                "simulation".to_string(),
            ],
        });
        reg.register_external_schema(ComponentSchema {
            name: "Position".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/position.json"))
                .unwrap(),
            modes: vec![
                "colony".to_string(),
                "roguelike".to_string(),
                "simulation".to_string(),
            ],
        });
    }
    let mut world = World::new(registry);
    world.current_mode = "colony".to_string();
    world
}

/// Helper: create a map with an optional non-transparent wall at (wx, wy).
fn map_with_wall(wx: i32, wy: i32, wall: bool) -> Map {
    let mut grid = SquareGridMap::new();
    for x in -10..=10 {
        for y in -10..=10 {
            grid.add_cell(x, y, 0);
        }
    }
    for x in -10..=10 {
        for y in -10..=10 {
            for dx in [-1, 0, 1] {
                for dy in [-1, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if (-10..=10).contains(&nx) && (-10..=10).contains(&ny) {
                        grid.add_neighbor((x, y, 0), (nx, ny, 0));
                    }
                }
            }
        }
    }
    let mut map = Map::new(Box::new(grid));
    if wall {
        map.set_cell_metadata(
            &CellKey::Square { x: wx, y: wy, z: 0 },
            serde_json::json!({"transparent": false}),
        );
    }
    map
}

/// Build a hex grid map for FOV testing (axial coordinates).
fn hex_open_plane(radius: i32) -> Map {
    let mut grid = HexGridMap::new();
    for q in -radius..=radius {
        for r in -radius..=radius {
            let s = -q - r;
            if s.abs() <= radius {
                grid.add_cell(q, r, 0);
            }
        }
    }
    let hex_dirs: [(i32, i32); 6] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, -1), (-1, 1)];
    let all_cells: Vec<CellKey> = grid.all_cells();
    for cell in &all_cells {
        if let CellKey::Hex { q, r, z } = cell {
            for &(dq, dr) in &hex_dirs {
                let nq = q + dq;
                let nr = r + dr;
                let _ns: i32 = -nq - nr;
                let neighbor = CellKey::Hex {
                    q: nq,
                    r: nr,
                    z: *z,
                };
                if grid.contains(&neighbor) {
                    let from = (*q, *r, *z);
                    let to = (nq, nr, *z);
                    grid.add_neighbor(from, to);
                }
            }
        }
    }
    Map::new(Box::new(grid))
}

/// Build a hex map with a single opaque wall at the given hex coordinate.
fn hex_map_with_wall(wq: i32, wr: i32, wall: bool) -> Map {
    let mut grid = HexGridMap::new();
    for q in -10..=10 {
        for r in -10..=10 {
            let s: i32 = -q - r;
            if s.abs() <= 10 {
                grid.add_cell(q, r, 0);
            }
        }
    }
    let hex_dirs: [(i32, i32); 6] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, -1), (-1, 1)];
    for q in -10..=10 {
        for r in -10..=10 {
            let s: i32 = -q - r;
            if s.abs() > 10 {
                continue;
            }
            for &(dq, dr) in &hex_dirs {
                let nq = q + dq;
                let nr = r + dr;
                let ns: i32 = -nq - nr;
                if ns.abs() <= 10 {
                    grid.add_neighbor((q, r, 0), (nq, nr, 0));
                }
            }
        }
    }
    let mut map = Map::new(Box::new(grid));
    if wall {
        map.set_cell_metadata(
            &CellKey::Hex { q: wq, r: wr, z: 0 },
            json!({"transparent": false}),
        );
    }
    map
}

// ====================================================================
// test_fog tests
// ====================================================================

#[test]
fn integration_fog_initially_empty() {
    let world = setup_world_with_schema();
    assert!(world.explored_cells.is_empty());
}

#[test]
fn integration_fog_get_none_for_nonexistent() {
    let world = setup_world_with_schema();
    assert_eq!(world.get_explored_cells(1), None);
}

#[test]
fn integration_fog_set_get_roundtrip() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    cells.insert(CellKey::Square { x: 1, y: 0, z: 0 });
    world.set_explored_cells(42, cells.clone());
    let retrieved = world.get_explored_cells(42);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), &cells);
}

#[test]
fn integration_reset_fog_clears_entity() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 5, y: 5, z: 0 });
    world.set_explored_cells(7, cells);
    assert!(world.get_explored_cells(7).is_some());
    world.reset_fog(7);
    assert_eq!(world.get_explored_cells(7), None);
}

#[test]
fn integration_reset_all_fog_clears_all() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    world.set_explored_cells(1, cells.clone());
    world.set_explored_cells(2, cells.clone());
    assert_eq!(world.explored_cells.len(), 2);
    world.reset_all_fog();
    assert!(world.explored_cells.is_empty());
}

#[test]
fn integration_fog_update_system_merges_visible_to_explored() {
    let mut world = setup_world_with_schema();
    world.map = Some(open_plane(10));

    let e = world.spawn_entity();
    world
        .set_component(e, "Sight", json!({"range": 5}))
        .unwrap();
    world
        .set_component(
            e,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();

    let mut fov = FovUpdateSystem;
    fov.run(&mut world);
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);

    let explored = world.get_explored_cells(e);
    assert!(explored.is_some());
    let cells = explored.unwrap();
    assert!(cells.contains(&CellKey::Square { x: 0, y: 0, z: 0 }));
    assert!(cells.len() > 1);
}

#[test]
fn integration_fog_accumulation_across_ticks() {
    let mut world = setup_world_with_schema();
    world.map = Some(open_plane(10));

    let e = world.spawn_entity();
    world
        .set_component(e, "Sight", json!({"range": 3}))
        .unwrap();
    world
        .set_component(
            e,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();

    let mut fov = FovUpdateSystem;
    fov.run(&mut world);
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);

    let explored_tick1 = world.get_explored_cells(e).unwrap().clone();
    assert!(explored_tick1.contains(&CellKey::Square { x: 3, y: 0, z: 0 }));

    world
        .set_component(
            e,
            "Position",
            json!({"pos": {"Square": {"x": 10, "y": 0, "z": 0}}}),
        )
        .unwrap();

    let mut fov = FovUpdateSystem;
    fov.run(&mut world);
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);

    let explored_tick2 = world.get_explored_cells(e).unwrap();
    assert!(explored_tick2.contains(&CellKey::Square { x: 3, y: 0, z: 0 }));
    assert!(explored_tick2.contains(&CellKey::Square { x: 10, y: 0, z: 0 }));
}

#[test]
fn integration_fog_serialization_roundtrip() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 10, y: 20, z: 0 });
    world.set_explored_cells(5, cells);

    let json = serde_json::to_string(&world).unwrap();
    let deserialized: World = serde_json::from_str(&json).unwrap();

    let retrieved = deserialized.get_explored_cells(5);
    assert!(retrieved.is_some());
    assert!(
        retrieved
            .unwrap()
            .contains(&CellKey::Square { x: 10, y: 20, z: 0 })
    );
}

#[test]
fn integration_fog_serialization_backward_compat() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 1, y: 2, z: 0 });
    world.set_explored_cells(3, cells);

    let mut json_value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
    if let Some(obj) = json_value.as_object_mut() {
        obj.remove("explored_cells");
    }
    let old_json = serde_json::to_string(&json_value).unwrap();

    let deserialized: World = serde_json::from_str(&old_json).unwrap();
    assert!(deserialized.explored_cells.is_empty());
}

#[test]
fn integration_no_sight_gets_no_fog() {
    let mut world = setup_world_with_schema();
    world.map = Some(open_plane(5));

    let e = world.spawn_entity();
    let mut fov = FovUpdateSystem;
    fov.run(&mut world);
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);

    assert!(world.get_explored_cells(e).is_none());
}

#[test]
fn integration_fog_system_name() {
    let system = FogUpdateSystem;
    assert_eq!(system.name(), "FogUpdateSystem");
}

#[test]
fn integration_fog_system_dependencies() {
    let system = FogUpdateSystem;
    assert_eq!(system.dependencies(), &["FovUpdateSystem"]);
}

#[test]
fn integration_fog_system_no_map() {
    let mut world = setup_world_with_schema();
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);
}

#[test]
fn integration_visibility_state_all_states() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 1, y: 0, z: 0 });
    world.set_explored_cells(1, cells);

    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 2, y: 0, z: 0 });
    world.set_visible_cells(1, visible);

    let cell_unexplored = CellKey::Square { x: 0, y: 0, z: 0 };
    let cell_explored = CellKey::Square { x: 1, y: 0, z: 0 };
    let cell_visible = CellKey::Square { x: 2, y: 0, z: 0 };

    assert_eq!(world.get_visibility_state(1, &cell_unexplored), 0);
    assert_eq!(world.get_visibility_state(1, &cell_explored), 1);
    assert_eq!(world.get_visibility_state(1, &cell_visible), 2);
}

#[test]
fn integration_fog_get_explored_cells_method() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 5, y: 5, z: 0 });
    world.set_explored_cells(10, cells.clone());
    assert_eq!(world.get_explored_cells(10), Some(&cells));
}

#[test]
fn integration_fog_set_explored_cells_method() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 3, y: 4, z: 0 });
    world.set_explored_cells(20, cells.clone());
    assert_eq!(world.get_explored_cells(20), Some(&cells));
}

#[test]
fn integration_fog_multiple_entities_independent() {
    let mut world = setup_world_with_schema();
    let mut cells_a = HashSet::new();
    cells_a.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    let mut cells_b = HashSet::new();
    cells_b.insert(CellKey::Square { x: 10, y: 10, z: 0 });

    world.set_explored_cells(100, cells_a);
    world.set_explored_cells(200, cells_b);

    assert_eq!(world.get_explored_cells(100).unwrap().len(), 1);
    assert_eq!(world.get_explored_cells(200).unwrap().len(), 1);
    assert!(
        !world
            .get_explored_cells(100)
            .unwrap()
            .contains(&CellKey::Square { x: 10, y: 10, z: 0 })
    );
}

// ====================================================================
// test_fov tests — Square FOV
// ====================================================================

#[test]
fn integration_origin_always_visible() {
    let map = open_plane(5);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(visible.contains(&origin));
}

#[test]
fn integration_range_zero_returns_empty() {
    let map = open_plane(5);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 0);
    assert!(visible.is_empty());
}

#[test]
fn integration_wall_blocks_los() {
    let mut grid = SquareGridMap::new();
    for x in -5..=5 {
        for y in -5..=5 {
            grid.add_cell(x, y, 0);
        }
    }
    for x in -5..=5 {
        for y in -5..=5 {
            for dx in [-1, 0, 1] {
                for dy in [-1, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if (-5..=5).contains(&nx) && (-5..=5).contains(&ny) {
                        grid.add_neighbor((x, y, 0), (nx, ny, 0));
                    }
                }
            }
        }
    }
    let mut map = Map::new(Box::new(grid));
    map.set_cell_metadata(
        &CellKey::Square { x: 1, y: 0, z: 0 },
        json!({"transparent": false}),
    );

    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);

    assert!(!visible.contains(&CellKey::Square { x: 2, y: 0, z: 0 }));
    assert!(visible.contains(&CellKey::Square { x: 2, y: 1, z: 0 }));
}

#[test]
fn integration_out_of_bounds_excluded() {
    let map = open_plane(3);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 10);
    assert!(!visible.contains(&CellKey::Square { x: 10, y: 0, z: 0 }));
}

#[test]
fn integration_fov_update_system_with_sight() {
    let mut world = setup_world_with_schema();
    world.map = Some(open_plane(10));

    let e = world.spawn_entity();
    world
        .set_component(e, "Sight", json!({"range": 5}))
        .unwrap();
    world
        .set_component(
            e,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();

    let mut system = FovUpdateSystem;
    system.run(&mut world);

    let visible = world.get_visible_cells(e);
    assert!(visible.is_some());
    let cells = visible.unwrap();
    assert!(cells.contains(&CellKey::Square { x: 0, y: 0, z: 0 }));
    assert!(cells.len() > 1);
}

#[test]
fn integration_fov_update_system_no_map() {
    let mut world = setup_world_with_schema();
    let mut system = FovUpdateSystem;
    system.run(&mut world);
}

#[test]
fn integration_fov_update_system_no_sight() {
    let mut world = setup_world_with_schema();
    world.map = Some(open_plane(5));

    let e = world.spawn_entity();
    let mut system = FovUpdateSystem;
    system.run(&mut world);

    assert!(world.get_visible_cells(e).is_none());
}

#[test]
fn integration_fov_update_system_dependencies() {
    let system = FovUpdateSystem;
    assert!(system.dependencies().is_empty());
}

#[test]
fn integration_visible_cells_get_none() {
    let world = setup_world_with_schema();
    assert_eq!(world.get_visible_cells(1), None);
}

#[test]
fn integration_visible_cells_set_get_roundtrip() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    world.set_visible_cells(42, cells.clone());
    let retrieved = world.get_visible_cells(42);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), &cells);
}

#[test]
fn integration_visible_cells_serde_skip() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    world.set_visible_cells(1, cells);

    let json = serde_json::to_string(&world).unwrap();
    let deserialized: World = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.get_visible_cells(1), None);
}

#[test]
fn integration_ring_shadow() {
    let mut grid = SquareGridMap::new();
    for x in -6..=6 {
        for y in -6..=6 {
            grid.add_cell(x, y, 0);
        }
    }
    for x in -6..=6 {
        for y in -6..=6 {
            for dx in [-1, 0, 1] {
                for dy in [-1, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if (-6..=6).contains(&nx) && (-6..=6).contains(&ny) {
                        grid.add_neighbor((x, y, 0), (nx, ny, 0));
                    }
                }
            }
        }
    }

    for pos in &[(3, 0), (-3, 0), (0, 3), (0, -3)] {
        grid.set_cell_metadata(
            &CellKey::Square {
                x: pos.0,
                y: pos.1,
                z: 0,
            },
            json!({"transparent": false}),
        );
    }

    let map = Map::new(Box::new(grid));
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 6);

    assert!(!visible.contains(&CellKey::Square { x: 4, y: 0, z: 0 }));
    assert!(!visible.contains(&CellKey::Square { x: -4, y: 0, z: 0 }));
    assert!(!visible.contains(&CellKey::Square { x: 0, y: 4, z: 0 }));
}

// Migrated inline map/fov.rs tests

#[test]
fn fov_open_plane_radius() {
    let map = open_plane(10);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let range = 5;
    let visible = compute_fov(&map, &origin, range);
    for d in 1..=range as i32 {
        assert!(visible.contains(&CellKey::Square { x: d, y: 0, z: 0 }));
        assert!(visible.contains(&CellKey::Square { x: -d, y: 0, z: 0 }));
        assert!(visible.contains(&CellKey::Square { x: 0, y: d, z: 0 }));
        assert!(visible.contains(&CellKey::Square { x: 0, y: -d, z: 0 }));
    }
    let beyond = range as i32 + 1;
    assert!(!visible.contains(&CellKey::Square {
        x: beyond,
        y: 0,
        z: 0
    }));
}

#[test]
fn fov_no_wall_does_not_block() {
    let map = map_with_wall(1, 0, false);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(visible.contains(&CellKey::Square { x: 2, y: 0, z: 0 }));
}

#[test]
fn fov_metadata_missing_defaults_transparent() {
    let map = open_plane(5);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(visible.contains(&CellKey::Square { x: 3, y: 0, z: 0 }));
}

#[test]
fn fov_wall_does_not_block_origin() {
    let map = map_with_wall(0, 0, true);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(visible.contains(&origin));
}

#[test]
fn fov_update_system_name() {
    let system = FovUpdateSystem;
    assert_eq!(system.name(), "FovUpdateSystem");
}

#[test]
fn fov_update_system_preserves_existing_no_sight() {
    let mut world = setup_world_with_schema();
    let map = open_plane(5);
    world.map = Some(map);

    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    world.set_visible_cells(99, cells);

    let mut system = FovUpdateSystem;
    system.run(&mut world);

    assert!(world.get_visible_cells(99).is_some());
}

#[test]
fn fov_ring_shadow_single_wall() {
    let map = map_with_wall(3, 0, true);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 6);

    assert!(!visible.contains(&CellKey::Square { x: 4, y: 0, z: 0 }));
    assert!(!visible.contains(&CellKey::Square { x: 5, y: 0, z: 0 }));
}

// ====================================================================
// test_fov tests — Hex FOV
// ====================================================================

#[test]
fn hex_origin_always_visible() {
    let map = hex_open_plane(5);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(visible.contains(&origin));
}

#[test]
fn hex_range_zero_returns_empty() {
    let map = hex_open_plane(5);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = BfsFovAlgorithm.compute_fov(&origin, 0, map.topology.as_ref());
    assert!(visible.is_empty());
}

#[test]
fn hex_open_plane_visibility() {
    let map = hex_open_plane(10);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let range = 4;
    let visible = compute_fov(&map, &origin, range);

    assert!(visible.contains(&origin));

    let neighbors = map.neighbors(&origin);
    for n in &neighbors {
        assert!(visible.contains(n));
    }

    assert!(visible.contains(&CellKey::Hex {
        q: range as i32,
        r: 0,
        z: 0
    }));

    let beyond = range as i32 + 1;
    assert!(!visible.contains(&CellKey::Hex {
        q: beyond,
        r: 0,
        z: 0
    }));
}

#[test]
fn hex_wall_is_visible() {
    let map = hex_map_with_wall(3, 0, true);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 6);

    assert!(visible.contains(&CellKey::Hex { q: 3, r: 0, z: 0 }));
}

#[test]
fn hex_wall_in_corridor_blocks() {
    let mut grid = HexGridMap::new();
    for q in 0..=5 {
        grid.add_cell(q, 0, 0);
    }
    for q in 0..5 {
        grid.add_neighbor((q, 0, 0), (q + 1, 0, 0));
    }
    let mut map = Map::new(Box::new(grid));

    map.set_cell_metadata(
        &CellKey::Hex { q: 2, r: 0, z: 0 },
        json!({"transparent": false}),
    );

    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);

    assert!(visible.contains(&CellKey::Hex { q: 2, r: 0, z: 0 }));
    assert!(!visible.contains(&CellKey::Hex { q: 3, r: 0, z: 0 }));
    assert!(!visible.contains(&CellKey::Hex { q: 4, r: 0, z: 0 }));
}

#[test]
fn hex_no_wall_does_not_block() {
    let map = hex_map_with_wall(2, 0, false);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(visible.contains(&CellKey::Hex { q: 3, r: 0, z: 0 }));
}

#[test]
fn hex_opaque_wall_visible() {
    let mut grid = HexGridMap::new();
    for q in 0..=3 {
        grid.add_cell(q, 0, 0);
    }
    for q in 0..3 {
        grid.add_neighbor((q, 0, 0), (q + 1, 0, 0));
    }
    let mut map = Map::new(Box::new(grid));

    map.set_cell_metadata(
        &CellKey::Hex { q: 1, r: 0, z: 0 },
        json!({"transparent": false}),
    );

    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 3);

    assert!(visible.contains(&CellKey::Hex { q: 1, r: 0, z: 0 }));
    assert!(!visible.contains(&CellKey::Hex { q: 2, r: 0, z: 0 }));
    assert!(!visible.contains(&CellKey::Hex { q: 3, r: 0, z: 0 }));
}
