use engine_core::ecs::ComponentRegistry;
use engine_core::ecs::World;
use engine_core::map::{CellKey, Map};
use engine_core::plugins::{EngineApi, load_plugin_and_register_worldgen};
use engine_core::worldgen::{
    ScriptingWorldgenPlugin, WorldgenError, WorldgenPlugin, WorldgenRegistry,
};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct DummyWorldgenPlugin;

impl ScriptingWorldgenPlugin for DummyWorldgenPlugin {
    fn invoke(
        &self,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        Ok(json!({
            "topology": "square",
            "cells": []
        }))
    }
    fn backend(&self) -> &str {
        "dummy"
    }
}

#[test]
fn test_register_and_list_worldgen_plugins() {
    let mut registry = WorldgenRegistry::new();

    // Simulate registering plugins from different sources
    registry.register(WorldgenPlugin::CAbi {
        name: "simple_square".to_string(),
        generate: Arc::new(|_| {
            json!({
                "topology": "square",
                "cells": []
            })
        }),
        _lib: None,
    });
    registry.register(WorldgenPlugin::Scripting {
        name: "cave_gen".to_string(),
        backend: "python".to_string(),
        opaque: Box::new(DummyWorldgenPlugin),
    });

    let names = registry.list_names();
    assert!(names.contains(&"simple_square".to_string()));
    assert!(names.contains(&"cave_gen".to_string()));
    assert_eq!(names.len(), 2);
}

#[test]
fn test_invoke_worldgen_plugin_returns_map() {
    let mut registry = WorldgenRegistry::new();

    registry.register(WorldgenPlugin::CAbi {
        name: "simple_square".to_string(),
        generate: Arc::new(|params: &serde_json::Value| {
            assert_eq!(params["width"], 10);
            json!({
                "topology": "square",
                "cells": [
                    { "x": 0, "y": 0, "z": 0, "neighbors": [] }
                ]
            })
        }),
        _lib: None,
    });

    let params = json!({ "width": 10, "height": 10, "seed": 42 });
    let map = registry
        .invoke("simple_square", &params)
        .expect("plugin should exist");

    assert!(map.get("cells").is_some());
    let cell = &map["cells"][0];
    assert_eq!(cell["x"], 0);
    assert_eq!(cell["y"], 0);
    assert_eq!(cell["z"], 0);
}

#[test]
fn test_register_and_list_lua_worldgen_plugin() {
    let mut registry = WorldgenRegistry::new();

    // Register a Lua function (mocked as a dummy plugin for core test)
    registry.register(WorldgenPlugin::Scripting {
        name: "hex_map".to_string(),
        backend: "lua".to_string(),
        opaque: Box::new(DummyWorldgenPlugin),
    });

    let names = registry.list_names();
    assert!(names.contains(&"hex_map".to_string()));
}

#[test]
fn test_invoke_nonexistent_plugin_returns_error() {
    let registry = WorldgenRegistry::new();
    let result = registry.invoke("nonexistent", &json!({}));
    assert!(matches!(result, Err(WorldgenError::NotFound)));
}

#[test]
fn test_register_and_invoke_cabi_worldgen_plugin() {
    let mut registry = WorldgenRegistry::new();

    let mut engine_api = EngineApi {
        spawn_entity: dummy_spawn_entity,
        set_component: dummy_set_component,
    };

    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut dummy_world = World::new(component_registry);
    let world: *mut std::os::raw::c_void = &mut dummy_world as *mut _ as *mut std::os::raw::c_void;

    let plugin_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root")
        .join("plugins")
        .join("simple_square_plugin")
        .join("libsimple_square_plugin.so");

    let result = unsafe {
        load_plugin_and_register_worldgen(plugin_path, &mut engine_api, world, &mut registry)
    };
    if let Err(e) = &result {
        eprintln!("Failed to load plugin: {e}");
    }
    assert!(result.is_ok(), "Plugin should load successfully");

    let names = registry.list_names();
    assert!(names.contains(&"simple_square".to_string()));

    let params = serde_json::json!({
        "width": 1,
        "height": 1,
        "z_levels": 1,
        "seed": 0,
        "chunk_x": 0,
        "chunk_y": 0
    });
    let map = registry
        .invoke("simple_square", &params)
        .expect("plugin should exist");
    assert!(map.get("cells").is_some());
    let cell = &map["cells"][0];
    assert_eq!(cell["x"], 0);
    assert_eq!(cell["y"], 0);
    assert_eq!(cell["z"], 0);
}

// Dummy engine API functions for testing
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

#[test]
fn test_map_from_json_square() {
    let value = json!({
        "topology": "square",
        "cells": [
            { "x": 0, "y": 0, "z": 0, "neighbors": [ { "x": 1, "y": 0, "z": 0 } ] },
            { "x": 1, "y": 0, "z": 0, "neighbors": [ { "x": 0, "y": 0, "z": 0 } ] }
        ]
    });
    let map = Map::from_json(&value).expect("should parse square map");
    assert_eq!(map.topology_type(), "square");
    assert!(map.contains(&CellKey::Square { x: 0, y: 0, z: 0 }));
    assert_eq!(
        map.neighbors(&CellKey::Square { x: 0, y: 0, z: 0 }),
        vec![CellKey::Square { x: 1, y: 0, z: 0 }]
    );
}

#[test]
fn test_map_from_json_hex() {
    let value = json!({
        "topology": "hex",
        "cells": [
            { "q": 0, "r": 0, "z": 0, "neighbors": [ { "q": 1, "r": 0, "z": 0 } ] },
            { "q": 1, "r": 0, "z": 0, "neighbors": [ { "q": 0, "r": 0, "z": 0 } ] }
        ]
    });
    let map = Map::from_json(&value).expect("should parse hex map");
    assert_eq!(map.topology_type(), "hex");
    assert!(map.contains(&CellKey::Hex { q: 0, r: 0, z: 0 }));
    assert_eq!(
        map.neighbors(&CellKey::Hex { q: 0, r: 0, z: 0 }),
        vec![CellKey::Hex { q: 1, r: 0, z: 0 }]
    );
}

#[test]
fn test_map_from_json_province() {
    let value = json!({
        "topology": "province",
        "cells": [
            { "id": "A", "neighbors": ["B"] },
            { "id": "B", "neighbors": ["A"] }
        ]
    });
    let map = Map::from_json(&value).expect("should parse province map");
    assert_eq!(map.topology_type(), "province");
    assert!(map.contains(&CellKey::Province {
        id: "A".to_string()
    }));
    assert_eq!(
        map.neighbors(&CellKey::Province {
            id: "A".to_string()
        }),
        vec![CellKey::Province {
            id: "B".to_string()
        }]
    );
}

#[test]
fn test_worldgen_plugin_schema_validation() {
    use engine_core::worldgen::{WorldgenPlugin, WorldgenRegistry};
    use serde_json::json;
    use std::sync::Arc;

    // Valid map (should succeed)
    let valid_map = json!({
        "topology": "square",
        "cells": [
            { "x": 0, "y": 0, "z": 0, "neighbors": [] }
        ]
    });

    // Invalid map (missing required "z" field)
    let invalid_map = json!({
        "topology": "square",
        "cells": [
            { "x": 0, "y": 0, "neighbors": [] }
        ]
    });

    let mut registry = WorldgenRegistry::new();

    // Plugin that returns valid map
    registry.register(WorldgenPlugin::CAbi {
        name: "valid_plugin".to_string(),
        generate: Arc::new(move |_| valid_map.clone()),
        _lib: None,
    });

    // Plugin that returns invalid map
    registry.register(WorldgenPlugin::CAbi {
        name: "invalid_plugin".to_string(),
        generate: Arc::new(move |_| invalid_map.clone()),
        _lib: None,
    });

    // Should succeed
    let result = registry.invoke("valid_plugin", &serde_json::json!({}));
    assert!(
        result.is_ok(),
        "Valid plugin output should pass schema validation"
    );

    // Should fail
    let result = registry.invoke("invalid_plugin", &serde_json::json!({}));
    assert!(
        result.is_err(),
        "Invalid plugin output should fail schema validation"
    );
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Map schema validation failed"),
        "Error should mention schema validation: {err}"
    );
}

#[test]
fn test_worldgen_custom_validator_and_postprocessor() {
    use engine_core::worldgen::{WorldgenPlugin, WorldgenRegistry};
    use serde_json::json;
    use std::sync::Arc;

    let mut registry = WorldgenRegistry::new();

    // Register a plugin that just returns a simple map
    registry.register(WorldgenPlugin::CAbi {
        name: "simple".to_string(),
        generate: Arc::new(|_| {
            json!({
                "topology": "square",
                "cells": [
                    { "x": 0, "y": 0, "z": 0, "neighbors": [] }
                ]
            })
        }),
        _lib: None,
    });

    // Register a validator that rejects maps with no cells
    registry.register_validator(|map| {
        let cells = map.get("cells").and_then(|v| v.as_array()).unwrap();
        if cells.is_empty() {
            Err("No cells".to_string())
        } else {
            Ok(())
        }
    });

    // Register a postprocessor that adds a marker field
    registry.register_postprocessor(|map| {
        map.as_object_mut()
            .unwrap()
            .insert("postprocessed".to_string(), json!(true));
    });

    let params = json!({});
    let result = registry.invoke("simple", &params).unwrap();
    assert_eq!(result["postprocessed"], true);
}

#[test]
fn test_worldgen_scripting_validator_and_postprocessor() {
    use engine_core::worldgen::{WorldgenPlugin, WorldgenRegistry};
    use serde_json::json;
    use std::sync::Arc;

    let mut registry = WorldgenRegistry::new();

    registry.register(WorldgenPlugin::CAbi {
        name: "simple2".to_string(),
        generate: Arc::new(|_| {
            json!({
                "topology": "square",
                "cells": [
                    { "x": 0, "y": 0, "z": 0, "neighbors": [] }
                ]
            })
        }),
        _lib: None,
    });

    // Register scripting validator
    registry.register_scripting_validator(|map| {
        if map.get("cells").unwrap().as_array().unwrap().len() != 1 {
            Err("Expected exactly one cell".to_string())
        } else {
            Ok(())
        }
    });

    // Register scripting postprocessor
    registry.register_scripting_postprocessor(|map| {
        map.as_object_mut()
            .unwrap()
            .insert("lua_post".to_string(), json!("ok"));
    });

    let params = json!({});
    let result = registry.invoke("simple2", &params).unwrap();
    assert_eq!(result["lua_post"], "ok");
}

#[test]
fn test_register_and_invoke_hex_worldgen_plugin() {
    use engine_core::worldgen::{WorldgenPlugin, WorldgenRegistry};
    use serde_json::json;

    let mut registry = WorldgenRegistry::new();

    registry.register(WorldgenPlugin::CAbi {
        name: "simple_hex".to_string(),
        generate: std::sync::Arc::new(|params: &serde_json::Value| {
            let width = params.get("width").and_then(|v| v.as_u64()).unwrap_or(1) as i32;
            let height = params.get("height").and_then(|v| v.as_u64()).unwrap_or(1) as i32;
            let z_levels = params.get("z_levels").and_then(|v| v.as_u64()).unwrap_or(1) as i32;

            let mut cells = Vec::new();

            // Axial neighbor offsets
            let neighbors_offset = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

            for z in 0..z_levels {
                for q in 0..width {
                    for r in 0..height {
                        let mut neighbors = Vec::new();
                        for (dq, dr) in neighbors_offset.iter() {
                            let nq = q + dq;
                            let nr = r + dr;
                            if nq >= 0 && nq < width && nr >= 0 && nr < height {
                                neighbors.push(json!({
                                    "q": nq,
                                    "r": nr,
                                    "z": z,
                                }));
                            }
                        }

                        cells.push(json!({
                            "q": q,
                            "r": r,
                            "z": z,
                            "neighbors": neighbors,
                            "biome": "TestBiome",
                            "terrain": "TestTerrain",
                        }));
                    }
                }
            }

            json!({
                "topology": "hex",
                "cells": cells,
            })
        }),
        _lib: None,
    });

    let params = json!({
        "width": 3,
        "height": 3,
        "z_levels": 1,
    });

    let map = registry
        .invoke("simple_hex", &params)
        .expect("Should generate map");

    assert_eq!(map["topology"], "hex");
    let cells = map["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 3 * 3 * 1);

    for cell in cells {
        assert!(cell.get("q").is_some());
        assert!(cell.get("r").is_some());
        assert!(cell.get("z").is_some());

        let neighbors = cell.get("neighbors").unwrap().as_array().unwrap();
        assert!(neighbors.len() <= 6);

        let biome = cell.get("biome").unwrap().as_str().unwrap();
        assert_eq!(biome, "TestBiome");

        let terrain = cell.get("terrain").unwrap().as_str().unwrap();
        assert_eq!(terrain, "TestTerrain");
    }
}

#[test]
fn test_register_and_invoke_province_worldgen_plugin() {
    use engine_core::worldgen::{WorldgenPlugin, WorldgenRegistry};
    use serde_json::json;

    let mut registry = WorldgenRegistry::new();

    registry.register(WorldgenPlugin::CAbi {
        name: "simple_province".to_string(),
        generate: std::sync::Arc::new(|_params: &serde_json::Value| {
            let cells = vec![
                json!({"id": "A", "neighbors": ["B", "C"]}),
                json!({"id": "B", "neighbors": ["A"]}),
                json!({"id": "C", "neighbors": ["A"]}),
            ];

            json!({
                "topology": "province",
                "cells": cells,
            })
        }),
        _lib: None,
    });

    let params = json!({});

    let map = registry
        .invoke("simple_province", &params)
        .expect("Should generate map");
    assert_eq!(map["topology"], "province");

    let cells = map["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 3);

    for cell in cells {
        // Validate presence of id and neighbors
        assert!(cell.get("id").is_some());
        let neighbors = cell.get("neighbors").unwrap().as_array().unwrap();
        for neighbor in neighbors {
            assert!(neighbor.is_string());
        }
    }
}
