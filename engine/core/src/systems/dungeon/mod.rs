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
                    cell_obj["metadata"] = json!({"walkable": false, "transparent": false});
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
