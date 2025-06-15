#[path = "helpers/worldgen.rs"]
mod worldgen_helper;
use worldgen_helper::setup_registry_with_c_plugin;

use engine_core::map::Map;
use serde_json::json;

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
