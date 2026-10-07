//! Cellular-caves map generation — random fill plus smoothing passes.
//!
//! Second in-core [`crate::worldgen::MapgenAlgorithm`] impl alongside
//! [`crate::systems::dungeon::DungeonGenerator`]: proves the switch/add paths
//! (switching is a name-string change, adding needs no core `match` edit).
//! Stateless utility, not an ECS System.

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for cellular-caves generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellularCavesConfig {
    /// Map width in cells.
    pub width: u32,
    /// Map height in cells.
    pub height: u32,
    /// RNG seed for deterministic output.
    pub seed: u64,
    /// Initial wall probability per interior cell.
    pub fill_chance: f64,
    /// Smoothing passes applied after the initial fill.
    pub steps: u32,
    /// Floor cell becomes wall when wall neighbors exceed this count.
    pub birth_limit: u32,
    /// Wall cell survives when wall neighbors reach this count.
    pub death_limit: u32,
}

impl CellularCavesConfig {
    /// Explicit-seed constructor for tests and scripted generation.
    /// Deterministic: the same inputs always produce the same map.
    pub fn explicit(width: u32, height: u32, seed: u64) -> Self {
        Self {
            width,
            height,
            seed,
            fill_chance: 0.45,
            steps: 4,
            birth_limit: 5,
            death_limit: 4,
        }
    }

    /// Build from invocation params; missing fields fall back to the
    /// explicit defaults. An explicit `seed` always wins over the
    /// time-seeded default.
    pub fn from_params(params: &serde_json::Value) -> Self {
        use crate::worldgen::{param_f64, param_u32, param_u64};
        let time_seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let defaults = Self::explicit(40, 25, time_seed);
        Self {
            width: param_u32(params, "width", defaults.width),
            height: param_u32(params, "height", defaults.height),
            seed: param_u64(params, "seed", time_seed),
            fill_chance: param_f64(params, "fill_chance", defaults.fill_chance),
            steps: param_u32(params, "steps", defaults.steps),
            birth_limit: param_u32(params, "birth_limit", defaults.birth_limit),
            death_limit: param_u32(params, "death_limit", defaults.death_limit),
        }
    }
}

/// Cellular-caves generator — stateless, pure function.
#[derive(Debug, Clone, Copy, Default)]
pub struct CellularCavesGenerator;

impl CellularCavesGenerator {
    /// Registry name this algorithm is invoked by.
    pub const NAME: &str = "caves";

    /// Generate the walkable floor set from the given config.
    /// Returns Err(String) if config is invalid (zero dimensions, etc.).
    pub fn generate_floor(config: &CellularCavesConfig) -> Result<HashSet<(u32, u32)>, String> {
        if config.width == 0 || config.height == 0 {
            return Err("Map dimensions must be positive".to_string());
        }
        if config.width < 3 || config.height < 3 {
            return Err("Map dimensions must be at least 3x3 for caves".to_string());
        }

        let mut rng = StdRng::seed_from_u64(config.seed);
        let (w, h) = (config.width, config.height);
        let mut walls = vec![vec![false; w as usize]; h as usize];
        for y in 0..h {
            for x in 0..w {
                walls[y as usize][x as usize] = x == 0
                    || y == 0
                    || x == w - 1
                    || y == h - 1
                    || rng.random::<f64>() < config.fill_chance;
            }
        }

        for _ in 0..config.steps {
            walls = smooth_step(&walls, w, h, config.birth_limit, config.death_limit);
        }

        let mut floor = HashSet::new();
        for y in 0..h {
            for x in 0..w {
                if !walls[y as usize][x as usize] {
                    floor.insert((x, y));
                }
            }
        }
        Ok(floor)
    }
}

/// One smoothing pass: walls survive crowded neighborhoods, floors cave in
/// under wall pressure. Out-of-bounds neighbors count as walls.
fn smooth_step(
    walls: &[Vec<bool>],
    w: u32,
    h: u32,
    birth_limit: u32,
    death_limit: u32,
) -> Vec<Vec<bool>> {
    let mut next = vec![vec![false; w as usize]; h as usize];
    for y in 0..h {
        for x in 0..w {
            let mut wall_count = 0u32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    let out_of_bounds = nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32;
                    if out_of_bounds || walls[ny as usize][nx as usize] {
                        wall_count += 1;
                    }
                }
            }
            next[y as usize][x as usize] = if walls[y as usize][x as usize] {
                wall_count >= death_limit
            } else {
                wall_count > birth_limit
            };
        }
    }
    next
}

impl crate::worldgen::MapgenAlgorithm for CellularCavesGenerator {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn generate(&self, params: &serde_json::Value) -> Result<serde_json::Value, String> {
        let config = CellularCavesConfig::from_params(params);
        let floor = Self::generate_floor(&config)?;
        Ok(
            crate::systems::dungeon::DungeonMap::from_floor_set(
                config.width,
                config.height,
                &floor,
            )
            .to_worldgen_json(),
        )
    }
}
