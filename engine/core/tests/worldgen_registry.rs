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
        eprintln!("Failed to load plugin: {}", e);
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
fn test_map_from_json_region() {
    let value = json!({
        "topology": "region",
        "cells": [
            { "id": "A", "neighbors": ["B"] },
            { "id": "B", "neighbors": ["A"] }
        ]
    });
    let map = Map::from_json(&value).expect("should parse region map");
    assert_eq!(map.topology_type(), "region");
    assert!(map.contains(&CellKey::Region {
        id: "A".to_string()
    }));
    assert_eq!(
        map.neighbors(&CellKey::Region {
            id: "A".to_string()
        }),
        vec![CellKey::Region {
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
        "Error should mention schema validation: {}",
        err
    );
}
