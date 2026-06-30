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
use engine_core::map::fov::compute_fov;
use engine_core::map::{Map, MapTopology, SquareGridMap};
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
                    if nx >= -10 && nx <= 10 && ny >= -10 && ny <= 10 {
                        grid.add_neighbor((x, y, 0), (nx, ny, 0));
                    }
                }
            }
        }
    }
    let mut map = Map::new(Box::new(grid));
    if wall {
        map.set_cell_metadata(
            &CellKey::Square {
                x: wx,
                y: wy,
                z: 0,
            },
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
            schema: serde_json::from_str(include_str!("../../assets/schemas/sight.json"))
                .unwrap(),
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
    assert!(
        visible.contains(&origin),
        "Origin must always be visible"
    );
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
                    if nx >= -5 && nx <= 5 && ny >= -5 && ny <= 5 {
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
        !visible.contains(&CellKey::Square {
            x: 10,
            y: 0,
            z: 0
        }),
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
    cells.insert(CellKey::Square {
        x: 0,
        y: 0,
        z: 0,
    });
    world.set_visible_cells(42, cells.clone());
    let retrieved = world.get_visible_cells(42);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), &cells);
}

#[test]
fn integration_visible_cells_serde_skip() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square {
        x: 0,
        y: 0,
        z: 0,
    });
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
                    if nx >= -6 && nx <= 6 && ny >= -6 && ny <= 6 {
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
        !visible.contains(&CellKey::Square {
            x: -4,
            y: 0,
            z: 0
        }),
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
        !visible.contains(&CellKey::Square { x: beyond, y: 0, z: 0 }),
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
    cells.insert(CellKey::Square {
        x: 0,
        y: 0,
        z: 0,
    });
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
