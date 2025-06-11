use engine_core::config::GameConfig;
use engine_core::map::Map;
use engine_core::plugins::loader::load_native_plugins_from_config;
use engine_core::plugins::types::EngineApi;
use engine_core::worldgen::WorldgenRegistry;
use serde_json::json;
use std::os::raw::{c_char, c_void};
use std::path::Path;

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

    // Load config and register all plugins listed there
    let config =
        GameConfig::load_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"))
            .expect("Failed to load config");

    unsafe { load_native_plugins_from_config(&config, &mut engine_api, world_ptr, &mut registry) }
        .expect("Failed to load native plugins from config");

    registry
}

#[test]
fn test_generate_and_apply_chunk() {
    let registry = setup_registry_with_c_plugin();

    // Simulate chunk parameters
    let chunk_params = json!({
        "width": 2,
        "height": 2,
        "z_levels": 1,
        "seed": 123,
        "chunk_x": 0,
        "chunk_y": 0
    });

    let chunk = registry
        .invoke("simple_square", &chunk_params)
        .expect("Chunk worldgen should succeed");

    // Validate chunk schema (should be handled by map_from_json)
    let map = Map::from_json(&chunk).expect("Chunk should parse as map");
    assert_eq!(map.all_cells().len(), 4);

    // Simulate merging another chunk
    let chunk2_params = json!({
        "width": 2,
        "height": 2,
        "z_levels": 1,
        "seed": 456,
        "chunk_x": 2,
        "chunk_y": 0
    });
    let chunk2 = registry
        .invoke("simple_square", &chunk2_params)
        .expect("Chunk2 worldgen should succeed");
    let map2 = Map::from_json(&chunk2).expect("Chunk2 should parse as map");

    // Merge logic: this will be implemented in Map
    let mut world_map = map;
    world_map.merge_chunk(&map2);

    assert_eq!(world_map.all_cells().len(), 8);
}

#[test]
fn test_schema_validation_rejects_invalid_map() {
    let invalid_map = json!({
        "topology": "square",
        "cells": [
            { "x": 0, "y": 0 } // missing "z" and "neighbors"
        ]
    });
    let result = Map::from_json(&invalid_map);
    assert!(result.is_err(), "Invalid map should be rejected");
}
