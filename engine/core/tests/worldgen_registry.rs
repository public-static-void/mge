use engine_core::ecs::ComponentRegistry;
use engine_core::ecs::World;
use engine_core::plugins::{EngineApi, load_plugin_and_register_worldgen};
use engine_core::worldgen::{WorldgenError, WorldgenPlugin, WorldgenRegistry};
use mlua::Lua;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_register_and_list_worldgen_plugins() {
    let mut registry = WorldgenRegistry::new();

    // Simulate registering plugins from different sources
    registry.register(WorldgenPlugin::CAbi {
        name: "simple_square".to_string(),
        generate: Box::new(|_| json!({ "cells": [] })),
    });
    registry.register(WorldgenPlugin::Python {
        name: "cave_gen".to_string(),
        generate: Box::new(|_| json!({ "cells": [] })),
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
        generate: Box::new(|params| {
            assert_eq!(params["width"], 10);
            json!({ "cells": [ { "id": "0,0", "x": 0, "y": 0 } ] })
        }),
    });

    let params = json!({ "width": 10, "height": 10, "seed": 42 });
    let map = registry
        .invoke("simple_square", &params)
        .expect("plugin should exist");

    assert!(map.get("cells").is_some());
    assert_eq!(map["cells"][0]["id"], "0,0");
}

#[test]
fn test_register_and_list_lua_worldgen_plugin() {
    let lua = Lua::new();
    let mut registry = WorldgenRegistry::new();

    // Register a Lua function
    let func = lua
        .create_function(|_, _params: mlua::Table| Ok(mlua::Value::Nil))
        .unwrap();
    let key = lua.create_registry_value(func).unwrap();

    registry.register(WorldgenPlugin::Lua {
        name: "hex_map".to_string(),
        registry_key: key,
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

    let params = serde_json::json!({});
    let map = registry
        .invoke("simple_square", &params)
        .expect("plugin should exist");
    assert!(map.get("cells").is_some());
    assert_eq!(map["cells"][0]["id"], "0,0");
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
