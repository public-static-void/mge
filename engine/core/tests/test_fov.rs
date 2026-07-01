//! Integration tests for the FOV system.
//!
//! Tests the core shadowcasting algorithm against known map patterns.
//! The FOV function is tested with various wall configurations to verify
//! correct line-of-sight blocking and shadow region formation.

use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::fov::BfsFovAlgorithm;
use engine_core::map::fov::FovAlgorithm;
use engine_core::map::fov::compute_fov;
use engine_core::map::{HexGridMap, Map, MapTopology, SquareGridMap};
use engine_core::systems::fov::FovUpdateSystem;
use serde_json::json;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Build an open plane map with 8-directional adjacency for FOV testing.
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

#[test]
fn integration_origin_always_visible() {
    let map = open_plane(5);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(visible.contains(&origin), "Origin must always be visible");
}

#[test]
fn integration_range_zero_returns_empty() {
    let map = open_plane(5);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 0);
    assert!(visible.is_empty(), "Range 0 should return empty set");
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
    // Place wall at (1, 0)
    map.set_cell_metadata(
        &CellKey::Square { x: 1, y: 0, z: 0 },
        json!({"transparent": false}),
    );

    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);

    // Cell behind wall should NOT be visible
    assert!(
        !visible.contains(&CellKey::Square { x: 2, y: 0, z: 0 }),
        "Cell (2,0) behind wall should not be visible"
    );
    // Off-axis cells should still be visible
    assert!(
        visible.contains(&CellKey::Square { x: 2, y: 1, z: 0 }),
        "Off-axis cell (2,1) should be visible"
    );
}

#[test]
fn integration_out_of_bounds_excluded() {
    let map = open_plane(3);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 10);
    assert!(
        !visible.contains(&CellKey::Square { x: 10, y: 0, z: 0 }),
        "Out-of-bounds cell should not be visible"
    );
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
    assert!(visible.is_some(), "Visible cells should be computed");
    let cells = visible.unwrap();
    assert!(
        cells.contains(&CellKey::Square { x: 0, y: 0, z: 0 }),
        "Origin must be visible"
    );
    assert!(cells.len() > 1, "Multiple cells should be visible");
}

#[test]
fn integration_fov_update_system_no_map() {
    let mut world = setup_world_with_schema();
    // No map set
    let mut system = FovUpdateSystem;
    system.run(&mut world);
    // Should not panic
}

#[test]
fn integration_fov_update_system_no_sight() {
    let mut world = setup_world_with_schema();
    world.map = Some(open_plane(5));

    let e = world.spawn_entity();
    // No Sight component — system should run without changing visible_cells
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
    // Create a ring of walls at distance 3 — cells behind the walls should be shadowed
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

    // Place walls in a plus pattern at distance 3
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

    // Cells behind the walls at distance 4+ on the same axis should NOT be visible
    assert!(
        !visible.contains(&CellKey::Square { x: 4, y: 0, z: 0 }),
        "Cell at (4,0) behind wall at (3,0) should be shadowed"
    );
    assert!(
        !visible.contains(&CellKey::Square { x: -4, y: 0, z: 0 }),
        "Cell at (-4,0) behind wall at (-3,0) should be shadowed"
    );
    assert!(
        !visible.contains(&CellKey::Square { x: 0, y: 4, z: 0 }),
        "Cell at (0,4) behind wall at (0,3) should be shadowed"
    );
}

// ---------------------------------------------------------------------------
// Tests migrated from inline `#[cfg(test)]` in map/fov.rs
// ---------------------------------------------------------------------------

#[test]
fn fov_open_plane_radius() {
    let map = open_plane(10);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let range = 5;
    let visible = compute_fov(&map, &origin, range);
    for d in 1..=range as i32 {
        assert!(
            visible.contains(&CellKey::Square { x: d, y: 0, z: 0 }),
            "Cell ({d}, 0) should be visible on open plane"
        );
        assert!(
            visible.contains(&CellKey::Square { x: -d, y: 0, z: 0 }),
            "Cell ({}, 0) should be visible on open plane",
            -d
        );
        assert!(
            visible.contains(&CellKey::Square { x: 0, y: d, z: 0 }),
            "Cell (0, {d}) should be visible on open plane"
        );
        assert!(
            visible.contains(&CellKey::Square { x: 0, y: -d, z: 0 }),
            "Cell (0, {}) should be visible on open plane",
            -d
        );
    }
    let beyond = range as i32 + 1;
    assert!(
        !visible.contains(&CellKey::Square {
            x: beyond,
            y: 0,
            z: 0
        }),
        "Cell ({beyond}, 0) beyond range should NOT be visible"
    );
}

#[test]
fn fov_no_wall_does_not_block() {
    let map = map_with_wall(1, 0, false);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(
        visible.contains(&CellKey::Square { x: 2, y: 0, z: 0 }),
        "Cell (2, 0) should be visible without wall at (1, 0)"
    );
}

#[test]
fn fov_metadata_missing_defaults_transparent() {
    let map = open_plane(5);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(
        visible.contains(&CellKey::Square { x: 3, y: 0, z: 0 }),
        "Cell with no metadata should default to transparent"
    );
}

#[test]
fn fov_wall_does_not_block_origin() {
    // Even if origin cell has block metadata, origin is always visible
    let map = map_with_wall(0, 0, true);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(
        visible.contains(&origin),
        "Origin must always be visible even if origin is a wall"
    );
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
    // Wall at (3, 0) should block (4, 0) and beyond
    let map = map_with_wall(3, 0, true);
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 6);

    assert!(
        !visible.contains(&CellKey::Square { x: 4, y: 0, z: 0 }),
        "Cell at (4,0) behind wall at (3,0) should be shadowed"
    );
    assert!(
        !visible.contains(&CellKey::Square { x: 5, y: 0, z: 0 }),
        "Cell at (5,0) behind wall at (3,0) should be shadowed"
    );
}

// ---------------------------------------------------------------------------
// Hex FOV tests
// ---------------------------------------------------------------------------

/// Build a hex grid map for FOV testing (axial coordinates).
fn hex_open_plane(radius: i32) -> Map {
    let mut grid = HexGridMap::new();
    // Add all hex cells in axial coordinates within the given radius
    for q in -radius..=radius {
        for r in -radius..=radius {
            let s = -q - r;
            if s.abs() <= radius {
                grid.add_cell(q, r, 0);
            }
        }
    }
    // Add 6-directional adjacency for all cells
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

#[test]
fn hex_origin_always_visible() {
    let map = hex_open_plane(5);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(
        visible.contains(&origin),
        "Hex origin must always be visible"
    );
}

#[test]
fn hex_range_zero_returns_empty() {
    let map = hex_open_plane(5);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = BfsFovAlgorithm.compute_fov(&origin, 0, map.topology.as_ref());
    assert!(visible.is_empty(), "Hex range 0 should return empty vec");
}

#[test]
fn hex_open_plane_visibility() {
    let map = hex_open_plane(10);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let range = 4;
    let visible = compute_fov(&map, &origin, range);

    // Origin should be visible
    assert!(visible.contains(&origin));

    // Immediate neighbors (distance 1) should be visible
    let neighbors = map.neighbors(&origin);
    for n in &neighbors {
        assert!(
            visible.contains(n),
            "Neighbor {:?} should be visible on open hex plane",
            n
        );
    }

    // Cells at distance 'range' on a straight line should be visible
    assert!(
        visible.contains(&CellKey::Hex {
            q: range as i32,
            r: 0,
            z: 0
        }),
        "Cell (range, 0) should be visible on open hex plane"
    );

    // Cells beyond range should NOT be visible
    let beyond = range as i32 + 1;
    assert!(
        !visible.contains(&CellKey::Hex {
            q: beyond,
            r: 0,
            z: 0
        }),
        "Cell beyond range should NOT be visible on hex plane"
    );
}

#[test]
fn hex_wall_is_visible() {
    let map = hex_map_with_wall(3, 0, true);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 6);

    // Wall itself should be visible
    assert!(
        visible.contains(&CellKey::Hex { q: 3, r: 0, z: 0 }),
        "Wall hex at (3,0) should be visible"
    );
}

#[test]
fn hex_wall_in_corridor_blocks() {
    // Create a narrow corridor (1-cell-wide line) with a wall in the middle
    // Cells beyond the wall should not be visible since there's no alternate path.
    let mut grid = HexGridMap::new();
    // Line of cells along q axis at r=0
    for q in 0..=5 {
        grid.add_cell(q, 0, 0);
    }
    // Only connect adjacent cells in a line (no off-axis connections)
    for q in 0..5 {
        grid.add_neighbor((q, 0, 0), (q + 1, 0, 0));
    }
    let mut map = Map::new(Box::new(grid));

    // Place wall at (2, 0)
    map.set_cell_metadata(
        &CellKey::Hex { q: 2, r: 0, z: 0 },
        json!({"transparent": false}),
    );

    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);

    // Wall itself is visible
    assert!(
        visible.contains(&CellKey::Hex { q: 2, r: 0, z: 0 }),
        "Wall hex should be visible"
    );

    // Cells beyond the wall (only reachable through the wall) should NOT be visible
    assert!(
        !visible.contains(&CellKey::Hex { q: 3, r: 0, z: 0 }),
        "Cell at (3,0) behind wall in corridor should not be visible"
    );
    assert!(
        !visible.contains(&CellKey::Hex { q: 4, r: 0, z: 0 }),
        "Cell at (4,0) behind wall in corridor should not be visible"
    );
}

#[test]
fn hex_no_wall_does_not_block() {
    let map = hex_map_with_wall(2, 0, false);
    let origin = CellKey::Hex { q: 0, r: 0, z: 0 };
    let visible = compute_fov(&map, &origin, 5);
    assert!(
        visible.contains(&CellKey::Hex { q: 3, r: 0, z: 0 }),
        "Cell (3,0) should be visible without wall at (2,0)"
    );
}

#[test]
fn hex_opaque_wall_visible() {
    // Wall blocks propagation: cells behind it in a corridor are hidden,
    // but the wall itself is visible
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

    // Wall is visible
    assert!(
        visible.contains(&CellKey::Hex { q: 1, r: 0, z: 0 }),
        "Opaque wall should be visible"
    );

    // Cell behind wall in corridor is not visible (cannot propagate through opaque)
    assert!(
        !visible.contains(&CellKey::Hex { q: 2, r: 0, z: 0 }),
        "Cell behind opaque wall in corridor should not be visible"
    );
    assert!(
        !visible.contains(&CellKey::Hex { q: 3, r: 0, z: 0 }),
        "Cell behind opaque wall in corridor should not be visible"
    );
}
