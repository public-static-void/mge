//! Procedural dungeon generation — produces rooms and L-shaped corridors
//! with seed-based determinism. Stateless utility, not an ECS System.

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

/// Map from cell coordinates to neighbor cell coordinate list.
type NeighborMap = HashMap<(u32, u32, u32), Vec<(u32, u32, u32)>>;

// ---- Data Structures -------------------------------------------------------

/// Configuration for dungeon generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonConfig {
    /// Map width in cells.
    pub width: u32,
    /// Map height in cells.
    pub height: u32,
    /// RNG seed for deterministic output.
    pub seed: u64,
    /// Minimum room width/height (inclusive).
    pub min_room_size: u32,
    /// Maximum room width/height (inclusive).
    pub max_room_size: u32,
    /// Maximum number of rooms to place.
    pub max_rooms: u32,
}

impl Default for DungeonConfig {
    fn default() -> Self {
        Self {
            width: 40,
            height: 25,
            seed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            min_room_size: 3,
            max_room_size: 8,
            max_rooms: 10,
        }
    }
}

/// A single cell in the dungeon map.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DungeonCell {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub walkable: bool,
}

/// A neighbor connection between two cells.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DungeonNeighbor {
    pub from_x: u32,
    pub from_y: u32,
    pub from_z: u32,
    pub to_x: u32,
    pub to_y: u32,
    pub to_z: u32,
}

/// The generated dungeon map.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DungeonMap {
    pub cells: Vec<DungeonCell>,
    pub neighbors: Vec<DungeonNeighbor>,
}

/// Room rectangle (internal).
#[derive(Debug, Clone)]
struct Room {
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
}

impl Room {
    fn center(&self) -> (u32, u32) {
        let cx = (self.x1 + self.x2) / 2;
        let cy = (self.y1 + self.y2) / 2;
        (cx, cy)
    }

    fn cells(&self) -> Vec<(u32, u32)> {
        let mut result = Vec::new();
        for x in self.x1..=self.x2 {
            for y in self.y1..=self.y2 {
                result.push((x, y));
            }
        }
        result
    }
}

/// Dungeon generator — stateless, pure function.
pub struct DungeonGenerator;

impl DungeonGenerator {
    /// Generate a dungeon map from the given config.
    /// Returns Err(String) if config is invalid (zero dimensions, etc.).
    pub fn generate(config: &DungeonConfig) -> Result<DungeonMap, String> {
        // Validate config
        if config.width == 0 || config.height == 0 {
            return Err("Map dimensions must be positive".to_string());
        }

        let mut rng = StdRng::seed_from_u64(config.seed);
        let rooms = generate_rooms(config, &mut rng);
        let corridors = generate_corridors(&rooms, config.width, config.height);
        Ok(build_dungeon_map(
            config.width,
            config.height,
            &rooms,
            &corridors,
        ))
    }
}

// ---- Room Placement ---------------------------------------------------------

/// Generate non-overlapping rooms within map bounds.
fn generate_rooms(config: &DungeonConfig, rng: &mut StdRng) -> Vec<Room> {
    if config.max_rooms == 0 {
        return Vec::new();
    }

    // Ensure min <= max by swapping if needed
    let min_size = config.min_room_size.min(config.max_room_size);
    let mut max_size = config.max_room_size.max(config.min_room_size);

    // Clamp max_room_size to fit within map (minus 2 for border walls)
    let max_fit = (config.width - 2).max(config.height - 2);
    if max_size > max_fit {
        max_size = max_fit;
    }

    if min_size > max_size || max_size == 0 {
        return Vec::new();
    }

    // Available area for rooms (1-cell border)
    let avail_w = (config.width - 2).saturating_sub(min_size);
    let avail_h = (config.height - 2).saturating_sub(min_size);

    if avail_w == 0 || avail_h == 0 {
        // Try to place a single room at minimum size
        let w = min_size.min(config.width - 2);
        let h = min_size.min(config.height - 2);
        if w < 2 || h < 2 {
            return Vec::new();
        }
        return vec![Room {
            x1: 1,
            y1: 1,
            x2: w,
            y2: h,
        }];
    }

    let mut rooms: Vec<Room> = Vec::new();
    let max_attempts = 200 * config.max_rooms as usize;
    let mut attempts = 0;

    while rooms.len() < config.max_rooms as usize && attempts < max_attempts {
        attempts += 1;

        let room_w = rng.random_range(min_size..=max_size.min(config.width - 2));
        let room_h = rng.random_range(min_size..=max_size.min(config.height - 2));

        // Position within [1, width-2] × [1, height-2] ensuring room fits
        let max_x = (config.width - 2).saturating_sub(room_w);
        let max_y = (config.height - 2).saturating_sub(room_h);
        if max_x == 0 || max_y == 0 {
            continue;
        }

        let x = rng.random_range(1..=max_x);
        let y = rng.random_range(1..=max_y);

        let new_room = Room {
            x1: x,
            y1: y,
            x2: x + room_w - 1,
            y2: y + room_h - 1,
        };

        // Check for overlap with existing rooms
        let overlaps = rooms.iter().any(|r| rooms_overlap(r, &new_room));
        if !overlaps {
            rooms.push(new_room);
        }
    }

    rooms
}

/// Check if two rooms overlap (axis-aligned rectangle intersection).
/// Includes a 1-cell padding gap between rooms.
fn rooms_overlap(a: &Room, b: &Room) -> bool {
    let a_x1 = a.x1.saturating_sub(1);
    let a_y1 = a.y1.saturating_sub(1);
    let a_x2 = a.x2.saturating_add(1);
    let a_y2 = a.y2.saturating_add(1);

    let b_x1 = b.x1.saturating_sub(1);
    let b_y1 = b.y1.saturating_sub(1);
    let b_x2 = b.x2.saturating_add(1);
    let b_y2 = b.y2.saturating_add(1);

    a_x1 < b_x2 && a_x2 > b_x1 && a_y1 < b_y2 && a_y2 > b_y1
}

// ---- Corridor Generation ----------------------------------------------------

/// Generate L-shaped (orthogonal) corridors connecting room centers sequentially.
/// Returns a set of (x, y) corridor cell coordinates.
fn generate_corridors(rooms: &[Room], width: u32, height: u32) -> HashSet<(u32, u32)> {
    let mut corridors = HashSet::new();
    if rooms.len() < 2 {
        return corridors;
    }

    for i in 0..rooms.len() - 1 {
        let (cx1, cy1) = rooms[i].center();
        let (cx2, cy2) = rooms[i + 1].center();

        // L-shaped path: horizontal then vertical
        // Horizontal segment: (cx1, cy1) -> (cx2, cy1)
        let x_min = cx1.min(cx2);
        let x_max = cx1.max(cx2);
        for x in x_min..=x_max {
            if x > 0 && x < width - 1 {
                corridors.insert((x, cy1));
            }
        }

        // Vertical segment: (cx2, cy1) -> (cx2, cy2)
        let y_min = cy1.min(cy2);
        let y_max = cy1.max(cy2);
        for y in y_min..=y_max {
            if cx2 > 0 && cx2 < width - 1 && y > 0 && y < height - 1 {
                corridors.insert((cx2, y));
            }
        }
    }

    corridors
}

// ---- Map Builder ------------------------------------------------------------

/// Build a complete DungeonMap from rooms and corridors.
fn build_dungeon_map(
    width: u32,
    height: u32,
    rooms: &[Room],
    corridors: &HashSet<(u32, u32)>,
) -> DungeonMap {
    // Build set of all floor cells (rooms + corridors)
    let mut floor_set: HashSet<(u32, u32)> = HashSet::new();

    for room in rooms {
        for (x, y) in room.cells() {
            floor_set.insert((x, y));
        }
    }

    for &(x, y) in corridors {
        floor_set.insert((x, y));
    }

    // Create all cells
    let mut cells = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let walkable = floor_set.contains(&(x, y));
            cells.push(DungeonCell {
                x,
                y,
                z: 0,
                walkable,
            });
        }
    }

    // Build neighbors for walkable cells (4-directional to adjacent walkable cells)
    let mut neighbors = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if !floor_set.contains(&(x, y)) {
                continue;
            }

            // Check 4 cardinal directions
            let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
            for (dx, dy) in dirs {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0
                    && nx < width as i32
                    && ny >= 0
                    && ny < height as i32
                    && floor_set.contains(&(nx as u32, ny as u32))
                {
                    neighbors.push(DungeonNeighbor {
                        from_x: x,
                        from_y: y,
                        from_z: 0,
                        to_x: nx as u32,
                        to_y: ny as u32,
                        to_z: 0,
                    });
                }
            }
        }
    }

    DungeonMap { cells, neighbors }
}

// ---- Worldgen Format Conversion ---------------------------------------------

impl DungeonMap {
    /// Convert DungeonMap to the worldgen JSON format expected by `apply_generated_map()`.
    /// Produces `{topology: "square", cells: [{x, y, z, neighbors: [{x, y, z}, ...], metadata: ...}]}`.
    ///
    /// Wall cells get `{walkable: false}` metadata so the pathfinder treats them as impassable.
    /// Floor cells have no metadata (default cost of 1.0).
    pub fn to_worldgen_json(&self) -> serde_json::Value {
        // Pre-index neighbors by (from_x, from_y, from_z) for O(1) lookup
        let mut neighbor_map: NeighborMap = HashMap::new();
        for n in &self.neighbors {
            neighbor_map
                .entry((n.from_x, n.from_y, n.from_z))
                .or_default()
                .push((n.to_x, n.to_y, n.to_z));
        }

        let cells_json: Vec<serde_json::Value> = self
            .cells
            .iter()
            .map(|cell| {
                let key = (cell.x, cell.y, cell.z);
                let neighs = neighbor_map.get(&key);

                let mut cell_obj = json!({
                    "x": cell.x,
                    "y": cell.y,
                    "z": cell.z,
                });

                // Add explicit neighbors if any
                if let Some(ns) = neighs {
                    let neigh_json: Vec<serde_json::Value> = ns
                        .iter()
                        .map(|(nx, ny, nz)| json!({"x": *nx, "y": *ny, "z": *nz}))
                        .collect();
                    cell_obj["neighbors"] = json!(neigh_json);
                }

                // Add walkable metadata for wall cells
                if !cell.walkable {
                    cell_obj["metadata"] = json!({"walkable": false});
                }

                cell_obj
            })
            .collect();

        json!({
            "topology": "square",
            "cells": cells_json,
        })
    }
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
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

        // Should have rooms (floor rectangles)
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

    // AC004 is covered by test_connectivity_all_rooms and test_wall_not_walkable.

    #[test]
    fn test_wall_not_walkable() {
        let config = default_config();
        let result = DungeonGenerator::generate(&config).unwrap();

        // AC005: Border cells are always walls
        for cell in &result.cells {
            if cell.x == 0
                || cell.x == config.width - 1
                || cell.y == 0
                || cell.y == config.height - 1
            {
                assert!(
                    !cell.walkable,
                    "Border cell ({},{}) should not be walkable",
                    cell.x, cell.y
                );
            }
        }

        // All non-floor cells should be not walkable
        // (verified implicitly by the generation logic)
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
    fn test_connectivity_all_rooms() {
        let config = default_config();
        let result = DungeonGenerator::generate(&config).unwrap();

        // Regenerate rooms to get their centers for connectivity test
        let mut rng = StdRng::seed_from_u64(config.seed);
        let rooms = generate_rooms(&config, &mut rng);

        if rooms.len() < 2 {
            // No connectivity test needed for 0-1 rooms
            return;
        }

        // Build adjacency from neighbor list
        let mut adj: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();
        for n in &result.neighbors {
            let from = (n.from_x, n.from_y);
            let to = (n.to_x, n.to_y);
            adj.entry(from).or_default().push(to);
            adj.entry(to).or_default().push(from); // bidirectional
        }

        // Union-Find over room centers to verify connectivity
        let room_centers: Vec<(u32, u32)> = rooms.iter().map(|r| r.center()).collect();
        let n_rooms = room_centers.len();
        let mut parent: Vec<usize> = (0..n_rooms).collect();

        fn find(parent: &mut Vec<usize>, x: usize) -> usize {
            if parent[x] != x {
                parent[x] = find(parent, parent[x]);
            }
            parent[x]
        }

        fn union(parent: &mut Vec<usize>, a: usize, b: usize) {
            let ra = find(parent, a);
            let rb = find(parent, b);
            if ra != rb {
                parent[ra] = rb;
            }
        }

        // BFS from each room center, union rooms reachable via shared paths
        for i in 0..n_rooms {
            let start = room_centers[i];
            let mut visited = HashSet::new();
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(start);
            visited.insert(start);

            while let Some(pos) = queue.pop_front() {
                for &neighbor in adj.get(&pos).unwrap_or(&vec![]) {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }

            // Union this room with any other room whose center is reachable
            for (j, center) in room_centers.iter().enumerate().skip(i + 1) {
                if visited.contains(center) {
                    union(&mut parent, i, j);
                }
            }
        }

        // All rooms should be in one component
        let root = find(&mut parent, 0);
        for i in 1..n_rooms {
            assert_eq!(
                find(&mut parent, i),
                root,
                "Room {i} should be connected to room 0"
            );
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
}
