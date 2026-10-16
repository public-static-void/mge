//! Planet-scale biome and climate assignment — generator-local synthesis of
//! per-cell temperature, humidity, and pressure plus the biome taxonomy.
//!
//! Stateless utility, not an ECS System. Follows the
//! [`crate::systems::cellular_caves`] single-file module idiom: one generator,
//! one config, pure helpers, no core `match` edits.
//!
//! Per-cell climate is a pure function of `(seed, x, y)` plus grid dimensions.
//! Each cell derives an independent sub-seed through an integer mixing step
//! (splitmix64 finalizer over the seed folded with the cell coordinates), so
//! the stream construction is order-independent: the same `(seed, x, y)`
//! yields the same climate triple regardless of iteration order, and no
//! `World` state (including `world.weather.rng_state`) is read or mutated.
//! Cross-process stability comes from the fixed integer mixer below — never
//! from the std hasher, whose keys are randomized per process.

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

use crate::worldgen::time_seed;

/// Registry name this algorithm is invoked by (bound in the next milestone).
pub const PLANET_BIOMES_NAME: &str = "planet_biomes";

/// Upper bound on generated cells; larger requests fail fast instead of
/// attempting a huge allocation.
pub const MAX_CELLS: i64 = 1_000_000;

/// Configuration for planet-biome generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanetBiomeConfig {
    /// RNG seed for deterministic output.
    pub seed: u64,
    /// Map width in cells.
    pub width: i32,
    /// Map height in cells.
    pub height: i32,
}

impl PlanetBiomeConfig {
    /// Default map width in cells.
    pub const DEFAULT_WIDTH: i32 = 32;
    /// Default map height in cells.
    pub const DEFAULT_HEIGHT: i32 = 32;

    /// Build from invocation params; missing `width`/`height` fall back to
    /// 32x32 and a missing `seed` falls back to [`time_seed`]. An explicit
    /// `seed` always wins over the time-seeded default. Unknown keys are
    /// ignored for forward compatibility. Non-integer, zero, negative, or
    /// over-cap dimensions fail with a message naming the offending key.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, String> {
        Self {
            seed: parse_seed(params)?,
            width: parse_dim(params, "width", Self::DEFAULT_WIDTH)?,
            height: parse_dim(params, "height", Self::DEFAULT_HEIGHT)?,
        }
        .checked()
    }

    fn checked(self) -> Result<Self, String> {
        let cells = self.width as i64 * self.height as i64;
        if cells > MAX_CELLS {
            return Err(format!(
                "width * height ({cells}) exceeds {MAX_CELLS} cells; \
                 reduce width ({}) or height ({})",
                self.width, self.height
            ));
        }
        Ok(self)
    }
}

/// Read the `seed` param: any JSON integer wins; anything else present but
/// non-numeric fails naming `seed`; absence falls back to [`time_seed`].
fn parse_seed(params: &serde_json::Value) -> Result<u64, String> {
    match params.get("seed") {
        None | Some(serde_json::Value::Null) => Ok(time_seed()),
        Some(v) => v
            .as_u64()
            .or_else(|| v.as_i64().and_then(|n| u64::try_from(n).ok()))
            .ok_or_else(|| format!("invalid seed {v}: expected a non-negative integer seed")),
    }
}

/// Read a dimension param: absent means `default`; present must be a JSON
/// integer in the positive `i32` range or the result names `key`.
fn parse_dim(params: &serde_json::Value, key: &str, default: i32) -> Result<i32, String> {
    match params.get(key) {
        None | Some(serde_json::Value::Null) => Ok(default),
        Some(v) => {
            // Reject floats/strings/bools/objects: only true JSON integers pass.
            let is_integer = v.is_u64() || v.is_i64();
            let parsed = v
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .filter(|n| *n > 0);
            match (is_integer, parsed) {
                (true, Some(n)) => Ok(n),
                _ => Err(format!(
                    "invalid {key} {v}: expected a positive integer dimension"
                )),
            }
        }
    }
}

/// Planet-biome generator — stateless handle carrying the registry name.
/// Implements [`crate::worldgen::MapgenAlgorithm`] below: `generate` parses
/// [`PlanetBiomeConfig`] from the invocation params and emits the square map
/// via [`generate_map`]. Pure climate model, taxonomy, and config parsing live
/// alongside it in this module.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlanetBiomeGenerator;

impl PlanetBiomeGenerator {
    /// Registry name this algorithm is invoked by.
    pub const NAME: &str = PLANET_BIOMES_NAME;
}

/// Mix `(seed, x, y)` into a per-cell sub-seed with a splitmix64 finalizer.
/// Fixed integer arithmetic keeps the mapping stable across processes and
/// toolchains; cell streams are independent of grid iteration order.
fn cell_subseed(seed: u64, x: i32, y: i32) -> u64 {
    let mut z = seed
        .wrapping_add((x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add((y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Latitude factor in `[0, 1]`: 0 on the equator row(s), 1 on the pole rows.
/// A single-row grid has no poles, so every cell is equatorial.
fn latitude01(y: i32, height: i32) -> f64 {
    if height <= 1 {
        return 0.0;
    }
    let half = f64::from(height - 1) / 2.0;
    ((f64::from(y) - half).abs() / half.max(0.5)).clamp(0.0, 1.0)
}

/// Synthesize per-cell `(temperature_c, humidity01, pressure_hpa)` as a pure
/// function of `(seed, x, y)` plus grid dimensions. Owns its RNG stream via
/// [`cell_subseed`]; reads and mutates no `World` state.
pub fn cell_climate(seed: u64, x: i32, y: i32, width: i32, height: i32) -> (f64, f64, f64) {
    let _ = width;
    let lat01 = latitude01(y, height);
    let mut rng = StdRng::seed_from_u64(cell_subseed(seed, x, y));
    let t_noise: f64 = rng.random_range(-5.0..=5.0);
    let h_noise: f64 = rng.random_range(-0.35..=0.35);
    let p_noise: f64 = rng.random_range(-15.0..=15.0);
    let elevation_drop: f64 = rng.random_range(0.0..=60.0);

    let temperature_c = (30.0 - 55.0 * lat01 + t_noise).clamp(-60.0, 60.0);
    let subtropical_dry = u8::from(lat01 > 0.25 && lat01 < 0.45) as f64;
    let humidity01 = (0.55 + h_noise - 0.15 * subtropical_dry).clamp(0.0, 1.0);
    let pressure_hpa = 1013.0 + p_noise - elevation_drop;
    (temperature_c, humidity01, pressure_hpa)
}

/// Map a `(temperature_c, humidity01, pressure_hpa)` triple to exactly one
/// biome string. RNG-free and total: every input returns a known label.
/// First match wins; temperature thresholds are lower-bound inclusive.
pub fn classify_biome(temperature_c: f64, humidity01: f64, pressure_hpa: f64) -> &'static str {
    if pressure_hpa < 980.0 {
        return "mountain";
    }
    if temperature_c < -8.0 {
        return "polar_ice";
    }
    if temperature_c < 2.0 {
        return "tundra";
    }
    if temperature_c < 10.0 {
        if humidity01 >= 0.45 {
            return "boreal_forest";
        }
        return "cold_steppe";
    }
    if temperature_c >= 28.0 && humidity01 < 0.25 {
        return "hot_desert";
    }
    if temperature_c >= 24.0 && humidity01 >= 0.65 {
        return "tropical_rainforest";
    }
    if temperature_c >= 20.0 && humidity01 < 0.4 {
        return "savanna";
    }
    if temperature_c >= 10.0 && humidity01 >= 0.55 {
        return "temperate_forest";
    }
    if temperature_c >= 10.0 && humidity01 >= 0.3 {
        return "temperate_grassland";
    }
    "barren"
}

/// Build the schema-valid `square` map JSON for `config`: every cell carries
/// integer `x`/`y`/`z = 0`, a `biome` label, and the full 4-neighbourhood.
/// Movement-relevant terrain rides on the top-level `terrain` slot (schema
/// surface) mirrored into `metadata.terrain` (runtime surface: the vehicle
/// guard and the presentation layer read terrain from cell metadata, which
/// `square_from_json` loads only from `metadata`). The mirror is a single
/// additive key — `walkable`/`transparent` stay unset so later merges and
/// postprocessors keep their defaults.
pub fn generate_map(config: &PlanetBiomeConfig) -> serde_json::Value {
    let mut cells = Vec::with_capacity(config.width as usize * config.height as usize);
    for y in 0..config.height {
        for x in 0..config.width {
            let (temperature_c, humidity01, pressure_hpa) =
                cell_climate(config.seed, x, y, config.width, config.height);
            let biome = classify_biome(temperature_c, humidity01, pressure_hpa);
            let mut cell = serde_json::json!({
                "x": x,
                "y": y,
                "z": 0,
                "biome": biome,
            });
            if biome == "mountain" {
                cell["terrain"] = serde_json::json!("mountain");
                cell["metadata"] = serde_json::json!({"terrain": "mountain"});
            }
            let mut neighbors = Vec::with_capacity(4);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && nx < config.width && ny >= 0 && ny < config.height {
                    neighbors.push(serde_json::json!({"x": nx, "y": ny, "z": 0}));
                }
            }
            cell["neighbors"] = serde_json::Value::Array(neighbors);
            cells.push(cell);
        }
    }
    serde_json::json!({
        "topology": "square",
        "cells": cells,
    })
}

impl crate::worldgen::MapgenAlgorithm for PlanetBiomeGenerator {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn generate(&self, params: &serde_json::Value) -> Result<serde_json::Value, String> {
        PlanetBiomeConfig::from_params(params).map(|config| generate_map(&config))
    }
}
