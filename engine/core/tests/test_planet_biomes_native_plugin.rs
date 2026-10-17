//! Integration test for the native planet-biome worldgen plugin.
//!
//! Verifies the plugin `.so` loads through the worldgen loader, registers under
//! a name distinct from the builtin, and emits maps that satisfy the full
//! invocation/emission contract — including byte-parity against the in-core
//! planet generator it delegates to.

use engine_core::ecs::{ComponentRegistry, World};
use engine_core::map::deserialize::validate_map_schema;
use engine_core::plugins::{EngineApi, load_plugin_and_register_worldgen};
use engine_core::systems::planet_biomes::PlanetBiomeGenerator;
use engine_core::worldgen::{MapgenAlgorithm, WorldgenError, WorldgenRegistry};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Registry name the plugin declares through `worldgen_name`.
const PLUGIN_NAME: &str = "planet_biomes_native";
/// Builtin name the plugin must not collide with.
const BUILTIN_NAME: &str = "planet_biomes";

/// Biome labels the generator's taxonomy can emit.
const ALLOWED_BIOMES: [&str; 11] = [
    "polar_ice",
    "tundra",
    "boreal_forest",
    "cold_steppe",
    "hot_desert",
    "tropical_rainforest",
    "savanna",
    "temperate_forest",
    "temperate_grassland",
    "mountain",
    "barren",
];

unsafe extern "C" fn dummy_spawn_entity(_world: *mut std::os::raw::c_void) -> u32 {
    0
}

unsafe extern "C" fn dummy_set_component(
    _world: *mut std::os::raw::c_void,
    _entity: u32,
    _name: *const std::os::raw::c_char,
    _json_value: *const std::os::raw::c_char,
) -> i32 {
    0
}

/// Resolve the workspace root by walking up from the crate manifest dir until
/// the `plugins/` directory is found.
fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("plugins").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing 'plugins' directory");
        }
    }
    dir
}

/// Path the loader convention expects for this plugin's built library.
fn plugin_path() -> PathBuf {
    workspace_root()
        .join("plugins")
        .join("planet_biomes_native_plugin")
        .join("libplanet_biomes_native_plugin.so")
}

/// Load the plugin into a fresh registry with no builtins registered.
fn load_into_fresh_registry() -> WorldgenRegistry {
    let path = plugin_path();
    assert!(
        path.exists(),
        "Plugin .so not found at {path:?}; run `make build-plugins` first"
    );

    let mut registry = WorldgenRegistry::new();
    let mut engine_api = EngineApi {
        spawn_entity: dummy_spawn_entity,
        set_component: dummy_set_component,
    };
    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(component_registry);
    let world_ptr: *mut std::os::raw::c_void = &mut world as *mut _ as *mut _;

    unsafe {
        load_plugin_and_register_worldgen(&path, &mut engine_api, world_ptr, &mut registry)
            .expect("planet_biomes_native plugin should load and register");
    }
    registry
}

/// Load the plugin into a fresh registry and invoke it, expecting success.
fn invoke_ok(params: &Value) -> Value {
    load_into_fresh_registry()
        .invoke(PLUGIN_NAME, params)
        .unwrap_or_else(|e| panic!("plugin invocation failed for {params}: {e:?}"))
}

/// Shared shape check: `map` is schema-valid, `square`, and carries exactly
/// `width * height` cells. Returns the failure reason instead of panicking so
/// the negative control can assert a rejection.
fn check_map_shape(map: &Value, width: i64, height: i64) -> Result<(), String> {
    validate_map_schema(map)?;
    if map.get("topology").and_then(Value::as_str) != Some("square") {
        return Err(format!(
            "topology must be square, got {:?}",
            map.get("topology")
        ));
    }
    let cells = map
        .get("cells")
        .and_then(Value::as_array)
        .ok_or_else(|| "cells must be an array".to_string())?;
    let expected = (width * height) as usize;
    if cells.len() != expected {
        return Err(format!("expected {expected} cells, got {}", cells.len()));
    }
    Ok(())
}

/// Count cells per biome label.
fn biome_histogram(map: &Value) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for cell in map["cells"].as_array().expect("cells array") {
        let biome = cell["biome"].as_str().expect("string biome");
        *counts.entry(biome.to_string()).or_insert(0) += 1;
    }
    counts
}

/// Index the 4-neighbourhood list of every cell by its `(x, y)` coordinate.
fn neighbors_by_coord(map: &Value) -> HashMap<(i64, i64), Vec<(i64, i64)>> {
    let mut out = HashMap::new();
    for cell in map["cells"].as_array().expect("cells array") {
        let x = cell["x"].as_i64().expect("integer x");
        let y = cell["y"].as_i64().expect("integer y");
        let list = cell["neighbors"]
            .as_array()
            .expect("neighbors array")
            .iter()
            .map(|n| {
                (
                    n["x"].as_i64().expect("integer neighbor x"),
                    n["y"].as_i64().expect("integer neighbor y"),
                )
            })
            .collect();
        assert!(
            out.insert((x, y), list).is_none(),
            "duplicate cell ({x}, {y})"
        );
    }
    out
}

// --- Load / registration (M1) ---

#[test]
fn plugin_registers_under_distinct_name() {
    let registry = load_into_fresh_registry();
    let names = registry.list_names();

    assert!(
        names.contains(&PLUGIN_NAME.to_string()),
        "expected plugin name {PLUGIN_NAME} in registry; got {names:?}"
    );
    assert!(
        !names.contains(&BUILTIN_NAME.to_string()),
        "builtin name {BUILTIN_NAME} must not be registered on a fresh registry; got {names:?}"
    );
}

// --- Parameter contract ---

#[test]
fn invocation_accepts_valid_params_and_rejects_invalid_dimensions() {
    let registry = load_into_fresh_registry();

    let ok = registry.invoke(PLUGIN_NAME, &json!({"seed": 42, "width": 32, "height": 32}));
    assert!(ok.is_ok(), "valid params must invoke: {:?}", ok.err());

    let zero_width = registry.invoke(PLUGIN_NAME, &json!({"width": 0}));
    assert!(
        zero_width.is_err(),
        "zero width must fail, got {zero_width:?}"
    );

    let over_cap = registry.invoke(PLUGIN_NAME, &json!({"width": 1, "height": 2000000}));
    assert!(
        over_cap.is_err(),
        "over-cap grid must fail, got {over_cap:?}"
    );
}

// --- Emission contract: count + schema ---

#[test]
fn emitted_map_has_expected_cell_count_and_valid_schema() {
    let map = invoke_ok(&json!({"seed": 42, "width": 32, "height": 32}));

    assert!(
        validate_map_schema(&map).is_ok(),
        "emitted map must satisfy the map schema"
    );
    check_map_shape(&map, 32, 32).expect("32x32 shape");
    assert_eq!(map["cells"].as_array().expect("cells array").len(), 1024);
}

// --- Emission contract: biomes, z-level, terrain mirror ---

#[test]
fn every_cell_has_known_biome_and_mountains_mirror_terrain() {
    let map = invoke_ok(&json!({"seed": 42, "width": 32, "height": 32}));
    let cells = map["cells"].as_array().expect("cells array");
    let mut mountains = 0;

    for cell in cells {
        let biome = cell["biome"].as_str().expect("string biome");
        assert!(!biome.is_empty(), "biome must be non-empty");
        assert!(ALLOWED_BIOMES.contains(&biome), "unknown biome {biome}");
        assert_eq!(cell["z"], 0);
        if biome == "mountain" {
            mountains += 1;
            assert_eq!(cell["terrain"], "mountain");
            assert_eq!(cell["metadata"]["terrain"], "mountain");
        } else {
            assert!(
                cell.get("terrain").is_none(),
                "non-mountain cell must omit terrain: {cell}"
            );
        }
    }
    assert!(mountains >= 1, "seed 42 must surface mountain coverage");
}

// --- Emission contract: 4-neighbourhood ---

#[test]
fn neighbour_lists_match_four_neighbourhood_expectations() {
    let map = invoke_ok(&json!({"seed": 7, "width": 8, "height": 8}));
    let neighbors = neighbors_by_coord(&map);
    assert_eq!(neighbors.len(), 64, "one entry per cell");

    assert_eq!(neighbors[&(0, 0)].len(), 2, "corner has 2 neighbors");
    assert_eq!(neighbors[&(7, 0)].len(), 2, "corner has 2 neighbors");
    assert_eq!(neighbors[&(0, 7)].len(), 2, "corner has 2 neighbors");
    assert_eq!(neighbors[&(7, 7)].len(), 2, "corner has 2 neighbors");
    assert_eq!(neighbors[&(4, 0)].len(), 3, "edge has 3 neighbors");
    assert_eq!(neighbors[&(0, 4)].len(), 3, "edge has 3 neighbors");
    assert_eq!(neighbors[&(4, 4)].len(), 4, "interior has 4 neighbors");

    for (pos, neighs) in &neighbors {
        for n in neighs {
            assert_ne!(*n, *pos, "no cell lists itself");
            assert!(
                (0..8).contains(&n.0) && (0..8).contains(&n.1),
                "neighbor {n:?} of {pos:?} stays in bounds"
            );
            assert_eq!(
                (n.0 - pos.0).abs() + (n.1 - pos.1).abs(),
                1,
                "neighbor {n:?} of {pos:?} must be 4-adjacent"
            );
        }
        let unique: HashSet<(i64, i64)> = neighs.iter().copied().collect();
        assert_eq!(
            unique.len(),
            neighs.len(),
            "no duplicate neighbors at {pos:?}"
        );
    }
}

// --- Value parity against the in-core oracle ---

#[test]
fn emitted_map_matches_in_core_generator_byte_for_byte() {
    let params = json!({"seed": 7, "width": 16, "height": 16});
    let map = invoke_ok(&params);
    let reference = PlanetBiomeGenerator
        .generate(&params)
        .expect("in-core reference generates");

    assert_eq!(
        serde_json::to_string(&map).unwrap(),
        serde_json::to_string(&reference).unwrap(),
        "plugin output must match the in-core generator byte for byte"
    );
}

// --- Determinism ---

#[test]
fn same_seed_is_byte_identical_and_distinct_seeds_diverge() {
    let params = json!({"seed": 12345, "width": 16, "height": 16});
    let first = invoke_ok(&params);
    let second = invoke_ok(&params);
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap(),
        "same seed must serialize byte-identically across invocations"
    );

    let seed1 = invoke_ok(&json!({"seed": 1, "width": 32, "height": 32}));
    let seed2 = invoke_ok(&json!({"seed": 2, "width": 32, "height": 32}));
    assert_ne!(
        biome_histogram(&seed1),
        biome_histogram(&seed2),
        "distinct seeds must not collapse to one constant output"
    );
}

// --- Negative control (non-vacuity guard) ---

#[test]
fn negative_controls_guard_against_a_vacuous_proof() {
    // (a) a fresh registry with no plugin loaded cannot resolve the name.
    let registry = WorldgenRegistry::new();
    match registry.invoke(PLUGIN_NAME, &json!({"seed": 1, "width": 4, "height": 4})) {
        Err(WorldgenError::NotFound { name, .. }) => assert_eq!(name, PLUGIN_NAME),
        other => panic!("expected NotFound for an unloaded registry, got {other:?}"),
    }

    // (b) a schema-valid but empty map still fails the shared shape check, so
    // the cell-count assertion is doing real work rather than passing vacuously.
    let empty = json!({"topology": "square", "cells": []});
    assert!(
        check_map_shape(&empty, 32, 32).is_err(),
        "an empty map must fail the shared shape check"
    );
}
