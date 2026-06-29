use engine_core::systems::dungeon::{DungeonCell, DungeonConfig, DungeonGenerator};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

fn default_config() -> DungeonConfig {
    DungeonConfig {
        width: 40,
        height: 25,
        seed: 42,
        min_room_size: 3,
        max_room_size: 8,
        max_rooms: 10,
    }
}

#[test]
fn test_generates_valid_map() {
    let config = default_config();
    let result = DungeonGenerator::generate(&config).unwrap();

    // AC001: Exactly width * height cells
    assert_eq!(result.cells.len(), (40 * 25) as usize);

    // Should have some walkable cells (rooms + corridors)
    let walkable_count = result.cells.iter().filter(|c| c.walkable).count();
    assert!(walkable_count > 0, "Map should have walkable cells");

    // Should have floor rectangles representing rooms
    let floor_rects = find_floor_rectangles(&result.cells, 40, 25);
    assert!(!floor_rects.is_empty(), "Map should have room rectangles");

    // Should have neighbors
    assert!(!result.neighbors.is_empty(), "Map should have neighbors");
}

#[test]
fn test_same_seed_identical() {
    let config = default_config();
    let a = DungeonGenerator::generate(&config).unwrap();
    let b = DungeonGenerator::generate(&config).unwrap();

    // AC002: Deep equality
    assert_eq!(a.cells, b.cells);
    assert_eq!(a.neighbors, b.neighbors);
}

#[test]
fn test_different_seeds_different() {
    let mut config_a = default_config();
    config_a.seed = 42;
    let mut config_b = default_config();
    config_b.seed = 99;

    let a = DungeonGenerator::generate(&config_a).unwrap();
    let b = DungeonGenerator::generate(&config_b).unwrap();

    // AC003: Different layouts (at least one cell differs in walkable)
    let a_walkable: Vec<(u32, u32, bool)> =
        a.cells.iter().map(|c| (c.x, c.y, c.walkable)).collect();
    let b_walkable: Vec<(u32, u32, bool)> =
        b.cells.iter().map(|c| (c.x, c.y, c.walkable)).collect();
    assert_ne!(a_walkable, b_walkable);
}

#[test]
fn test_wall_not_walkable() {
    let config = default_config();
    let result = DungeonGenerator::generate(&config).unwrap();

    // AC005: Border cells are always walls
    for cell in &result.cells {
        if cell.x == 0 || cell.x == config.width - 1 || cell.y == 0 || cell.y == config.height - 1 {
            assert!(
                !cell.walkable,
                "Border cell ({},{}) should not be walkable",
                cell.x, cell.y
            );
        }
    }
}

#[test]
fn test_invalid_config_error() {
    let config = DungeonConfig {
        width: 0,
        height: 0,
        ..default_config()
    };
    let result = DungeonGenerator::generate(&config);
    assert!(result.is_err(), "Zero dimensions should return error");
}

#[test]
fn test_max_rooms_zero() {
    let config = DungeonConfig {
        max_rooms: 0,
        ..default_config()
    };
    let result = DungeonGenerator::generate(&config).unwrap();

    // EC-02: All-wall map
    let walkable_count = result.cells.iter().filter(|c| c.walkable).count();
    assert_eq!(walkable_count, 0, "max_rooms=0 should produce all-wall map");
}

#[test]
fn test_min_greater_than_max() {
    let config = DungeonConfig {
        min_room_size: 10,
        max_room_size: 3,
        ..default_config()
    };
    let result = DungeonGenerator::generate(&config).unwrap();

    // EC-08: Should not crash, should generate rooms
    let walkable_count = result.cells.iter().filter(|c| c.walkable).count();
    assert!(
        walkable_count > 0,
        "min>max room sizes should still produce walkable cells"
    );
}

#[test]
fn test_connectivity_all_walkable_cells() {
    let config = default_config();
    let result = DungeonGenerator::generate(&config).unwrap();

    // Build adjacency from neighbor list
    let mut adj: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();
    for n in &result.neighbors {
        let from = (n.from_x, n.from_y);
        let to = (n.to_x, n.to_y);
        adj.entry(from).or_default().push(to);
        adj.entry(to).or_default().push(from);
    }

    // Collect all walkable cell coordinates
    let walkable: Vec<(u32, u32)> = result
        .cells
        .iter()
        .filter(|c| c.walkable)
        .map(|c| (c.x, c.y))
        .collect();

    // Verify all walkable cells form a single connected component
    if let Some(start) = walkable.first() {
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(*start);
        visited.insert(*start);

        while let Some(pos) = queue.pop_front() {
            if let Some(neighbors) = adj.get(&pos) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        for cell in &walkable {
            assert!(
                visited.contains(cell),
                "Walkable cell ({},{}) should be reachable from all other walkable cells",
                cell.0,
                cell.1
            );
        }
    }
}

#[test]
fn test_performance_budget() {
    let config = default_config();
    let start = Instant::now();

    for _ in 0..10 {
        DungeonGenerator::generate(&config).unwrap();
    }

    let elapsed = start.elapsed();
    let per_call = elapsed / 10;
    assert!(
        per_call < std::time::Duration::from_millis(100),
        "Generation too slow: {:?} per call",
        per_call
    );
}

#[test]
fn test_large_map_performance() {
    let config = DungeonConfig {
        width: 100,
        height: 100,
        seed: 42,
        max_rooms: 30,
        ..Default::default()
    };

    let start = Instant::now();
    let result = DungeonGenerator::generate(&config).unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "100x100 map generation too slow: {:?}",
        elapsed
    );
    assert_eq!(result.cells.len(), 10000);
}

#[test]
fn test_to_worldgen_json() {
    let config = default_config();
    let map = DungeonGenerator::generate(&config).unwrap();
    let json = map.to_worldgen_json();

    // Verify structure
    assert_eq!(json["topology"], "square");
    assert!(json["cells"].is_array());

    let cells = json["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 1000);

    // Each cell should have x, y, z
    for cell in cells {
        assert!(cell.get("x").is_some());
        assert!(cell.get("y").is_some());
        assert!(cell.get("z").is_some());
    }

    // Some wall cells should have walkable=false metadata
    let wall_cells: Vec<&serde_json::Value> = cells
        .iter()
        .filter(|c| {
            c.get("metadata")
                .and_then(|m| m.get("walkable"))
                .and_then(|w| w.as_bool())
                == Some(false)
        })
        .collect();
    assert!(!wall_cells.is_empty(), "Should have some wall cells");
}

#[test]
fn test_single_room() {
    let config = DungeonConfig {
        max_rooms: 1,
        ..default_config()
    };
    let result = DungeonGenerator::generate(&config).unwrap();

    // EC-10: Single room, no corridors
    let walkable_count = result.cells.iter().filter(|c| c.walkable).count();
    assert!(
        walkable_count > 0,
        "Single room should produce walkable cells"
    );
}

// ---- Helpers ------------------------------------------------------------

/// Find contiguous rectangles of floor cells (simple heuristic).
fn find_floor_rectangles(
    cells: &[DungeonCell],
    width: u32,
    height: u32,
) -> Vec<(u32, u32, u32, u32)> {
    let walkable: HashSet<(u32, u32)> = cells
        .iter()
        .filter(|c| c.walkable)
        .map(|c| (c.x, c.y))
        .collect();

    let mut visited = HashSet::new();
    let mut rects = Vec::new();

    for y in 0..height {
        for x in 0..width {
            if walkable.contains(&(x, y)) && !visited.contains(&(x, y)) {
                // Flood fill to find connected region
                let mut region = Vec::new();
                let mut queue = std::collections::VecDeque::new();
                queue.push_back((x, y));
                visited.insert((x, y));

                while let Some((cx, cy)) = queue.pop_front() {
                    region.push((cx, cy));
                    for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                        let nx = cx as i32 + dx;
                        let ny = cy as i32 + dy;
                        if nx >= 0
                            && nx < width as i32
                            && ny >= 0
                            && ny < height as i32
                            && walkable.contains(&(nx as u32, ny as u32))
                            && !visited.contains(&(nx as u32, ny as u32))
                        {
                            visited.insert((nx as u32, ny as u32));
                            queue.push_back((nx as u32, ny as u32));
                        }
                    }
                }

                // Compute bounding box
                if !region.is_empty() {
                    let min_x = region.iter().map(|p| p.0).min().unwrap();
                    let max_x = region.iter().map(|p| p.0).max().unwrap();
                    let min_y = region.iter().map(|p| p.1).min().unwrap();
                    let max_y = region.iter().map(|p| p.1).max().unwrap();
                    rects.push((min_x, min_y, max_x, max_y));
                }
            }
        }
    }

    rects
}
