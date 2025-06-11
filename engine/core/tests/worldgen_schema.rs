use engine_core::plugins::loader::load_plugin_and_register_worldgen;
use engine_core::plugins::types::EngineApi;
use engine_core::worldgen::{WorldgenPlugin, WorldgenRegistry};
use serde_json::{Value, json};
use std::os::raw::{c_char, c_void};
use std::path::PathBuf;
use std::sync::Arc;

/// Minimal C ABI-compatible spawn_entity function for testing
unsafe extern "C" fn test_spawn_entity(_world: *mut c_void) -> u32 {
    0
}
unsafe extern "C" fn test_set_component(
    _world: *mut c_void,
    _entity: u32,
    _name: *const c_char,
    _json_value: *const c_char,
) -> i32 {
    0
}

fn setup_registry_with_c_plugin() -> WorldgenRegistry {
    let mut registry = WorldgenRegistry::new();

    let mut engine_api = EngineApi {
        spawn_entity: test_spawn_entity,
        set_component: test_set_component,
    };
    let world_ptr = std::ptr::null_mut();

    // Find workspace root by going up until we see a "plugins" directory
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("plugins").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing 'plugins' directory");
        }
    }
    let plugin_path = dir.join("plugins/simple_square_plugin/libsimple_square_plugin.so");

    assert!(
        plugin_path.exists(),
        "Plugin .so not found at {:?}. CWD: {:?}",
        plugin_path,
        std::env::current_dir().unwrap()
    );

    unsafe {
        load_plugin_and_register_worldgen(
            plugin_path.to_str().unwrap(),
            &mut engine_api,
            world_ptr,
            &mut registry,
        )
        .expect("Failed to load C plugin");
    }
    registry
}

#[test]
fn test_register_and_invoke_rust_worldgen_plugin() {
    let params = json!({
        "topology": "square",
        "width": 4,
        "height": 3,
        "z_levels": 1,
        "seed": 123
    });

    let plugin = WorldgenPlugin::CAbi {
        name: "test_square_worldgen".to_string(),
        generate: Arc::new(|params: &Value| {
            let width = params.get("width").and_then(|v| v.as_u64()).unwrap() as i32;
            let height = params.get("height").and_then(|v| v.as_u64()).unwrap() as i32;
            let z_levels = params.get("z_levels").and_then(|v| v.as_u64()).unwrap() as i32;

            let mut cells = Vec::new();
            for z in 0..z_levels {
                for y in 0..height {
                    for x in 0..width {
                        cells.push(json!({
                            "x": x,
                            "y": y,
                            "z": z,
                            "neighbors": []
                        }));
                    }
                }
            }
            json!({
                "topology": "square",
                "cells": cells
            })
        }),
        _lib: None,
    };

    let mut registry = WorldgenRegistry::new();
    registry.register(plugin);

    let result = registry
        .invoke("test_square_worldgen", &params)
        .expect("Worldgen plugin should succeed");

    assert_eq!(result.get("topology").unwrap(), "square");
    let cells = result.get("cells").unwrap().as_array().unwrap();
    assert_eq!(cells.len(), 4 * 3);
    let first = &cells[0];
    assert_eq!(first.get("x").unwrap(), 0);
    assert_eq!(first.get("y").unwrap(), 0);
    assert_eq!(first.get("z").unwrap(), 0);
}

#[test]
fn test_basic_square_worldgen_plugin() {
    let params = json!({
        "topology": "square",
        "width": 5,
        "height": 3,
        "z_levels": 1,
        "seed": 42
    });

    let registry = setup_registry_with_c_plugin();

    let result = registry.invoke("simple_square", &params);

    assert!(result.is_ok(), "Expected worldgen plugin to succeed");
    let map = result.unwrap();
    assert_eq!(map.get("topology").unwrap(), "square");
    let cells = map.get("cells").unwrap().as_array().unwrap();
    assert_eq!(cells.len(), 5 * 3);
}

#[test]
fn test_basic_square_worldgen_with_terrain_and_biomes() {
    let params = json!({
        "topology": "square",
        "width": 4,
        "height": 3,
        "z_levels": 1,
        "seed": 123,
        "biomes": [
            { "name": "Plains", "tiles": ["grass", "dirt"] },
            { "name": "Forest", "tiles": ["tree", "grass"] }
        ]
    });

    let registry = setup_registry_with_c_plugin();

    let result = registry.invoke("simple_square", &params);
    assert!(result.is_ok(), "Expected worldgen plugin to succeed");
    let map = result.unwrap();

    assert_eq!(map.get("topology").unwrap(), "square");
    let cells = map.get("cells").unwrap().as_array().unwrap();
    assert_eq!(cells.len(), 4 * 3);

    for cell in cells {
        let terrain = cell.get("terrain").expect("Cell missing terrain");
        let biome = cell.get("biome").expect("Cell missing biome");
        assert!(terrain.is_string(), "Terrain must be a string");
        assert!(biome.is_string(), "Biome must be a string");
        let biome_str = biome.as_str().unwrap();
        assert!(biome_str == "Plains" || biome_str == "Forest");
    }
}

#[test]
fn test_basic_square_worldgen_with_neighbors() {
    let params = serde_json::json!({
        "topology": "square",
        "width": 3,
        "height": 2,
        "z_levels": 1,
        "seed": 1,
        "biomes": [
            { "name": "Plains", "tiles": ["grass"] }
        ]
    });

    let registry = setup_registry_with_c_plugin();

    let result = registry.invoke("simple_square", &params);
    assert!(result.is_ok(), "Expected worldgen plugin to succeed");
    let map = result.unwrap();
    assert_eq!(map.get("topology").unwrap(), "square");
    let cells = map.get("cells").unwrap().as_array().unwrap();
    assert_eq!(cells.len(), 3 * 2);

    use std::collections::HashSet;
    let positions: HashSet<(i32, i32, i32)> = cells
        .iter()
        .map(|cell| {
            (
                cell.get("x").unwrap().as_i64().unwrap() as i32,
                cell.get("y").unwrap().as_i64().unwrap() as i32,
                cell.get("z").unwrap().as_i64().unwrap() as i32,
            )
        })
        .collect();

    let mut found_nonempty = false;
    for cell in cells {
        let x = cell.get("x").unwrap().as_i64().unwrap() as i32;
        let y = cell.get("y").unwrap().as_i64().unwrap() as i32;
        let z = cell.get("z").unwrap().as_i64().unwrap() as i32;
        let neighbors = cell.get("neighbors").unwrap().as_array().unwrap();

        if !neighbors.is_empty() {
            found_nonempty = true;
        }

        for n in neighbors {
            let nx = n.get("x").unwrap().as_i64().unwrap() as i32;
            let ny = n.get("y").unwrap().as_i64().unwrap() as i32;
            let nz = n.get("z").unwrap().as_i64().unwrap() as i32;

            assert!(positions.contains(&(nx, ny, nz)), "Neighbor not in map");
            let dist = (x - nx).abs() + (y - ny).abs() + (z - nz).abs();
            assert_eq!(dist, 1, "Neighbor not adjacent");
        }
    }
    assert!(
        found_nonempty,
        "No cell had any neighbors; neighbor generation is missing!"
    );
}
