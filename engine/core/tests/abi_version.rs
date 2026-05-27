//! Integration tests for plugin ABI versioning.
//!
//! Verifies:
//! - AC001: Matching version loads successfully
//! - AC002: Mismatched version is rejected
//! - AC003: Version check precedes init (structural, verified by test flow)
//! - AC004: Error is returned, not panicked
//! - AC005: Pre-versioning plugin (abi_version=0) is rejected
//! - AC007: Error message contains path, expected, actual
//! - AC008: All 5 loader functions perform the check

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Resolve workspace root from CARGO_MANIFEST_DIR.
fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("plugins").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing 'plugins' directory");
        }
    }
    dir
}

/// Resolve a plugin `.so` path relative to the workspace root.
fn plugin_path(name: &str) -> PathBuf {
    workspace_root()
        .join("plugins")
        .join(name)
        .join(format!("lib{name}.so"))
}

fn make_engine_api() -> engine_core::plugins::EngineApi {
    engine_core::plugins::EngineApi {
        spawn_entity: engine_core::plugins::ffi_spawn_entity,
        set_component: engine_core::plugins::ffi_set_component,
    }
}

fn make_world() -> *mut c_void {
    let mut registry = engine_core::ecs::registry::ComponentRegistry::new();
    let schema_json = r#"{
        "title": "Position",
        "type": "object",
        "properties": {
            "x": { "type": "number" },
            "y": { "type": "number" }
        },
        "required": ["x", "y"],
        "modes": ["colony", "roguelike"]
    }"#;
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));
    let world = Box::new(engine_core::ecs::World::new(registry.clone()));
    Box::into_raw(world) as *mut c_void
}

// ---------------------------------------------------------------------------
// AC001: Matching version loads successfully
// ---------------------------------------------------------------------------

#[test]
fn test_matching_version_loads_test_plugin() {
    let path = plugin_path("test_plugin");
    assert!(path.exists(), "Plugin .so not found at {path:?}");
    let mut api = make_engine_api();
    let world_ptr = make_world();
    unsafe {
        engine_core::plugins::load_plugin(&path, &mut api, world_ptr)
            .expect("test_plugin should load successfully");
    }
}

#[test]
fn test_matching_version_loads_simple_square() {
    let path = plugin_path("simple_square_plugin");
    assert!(path.exists(), "Plugin .so not found at {path:?}");
    let mut api = make_engine_api();
    let world_ptr = make_world();
    unsafe {
        engine_core::plugins::load_plugin(&path, &mut api, world_ptr)
            .expect("simple_square_plugin should load successfully");
    }
}

#[test]
fn test_matching_version_loads_simple_hex() {
    let path = plugin_path("simple_hex_plugin");
    assert!(path.exists(), "Plugin .so not found at {path:?}");
    let mut api = make_engine_api();
    let world_ptr = make_world();
    unsafe {
        engine_core::plugins::load_plugin(&path, &mut api, world_ptr)
            .expect("simple_hex_plugin should load successfully");
    }
}

#[test]
fn test_matching_version_loads_simple_province() {
    let path = plugin_path("simple_province_plugin");
    assert!(path.exists(), "Plugin .so not found at {path:?}");
    let mut api = make_engine_api();
    let world_ptr = make_world();
    unsafe {
        engine_core::plugins::load_plugin(&path, &mut api, world_ptr)
            .expect("simple_province_plugin should load successfully");
    }
}

#[test]
fn test_matching_version_loads_rust_test_plugin() {
    let path = plugin_path("rust_test_plugin");
    assert!(path.exists(), "Plugin .so not found at {path:?}");
    let mut api = make_engine_api();
    let world_ptr = make_world();
    unsafe {
        engine_core::plugins::load_plugin(&path, &mut api, world_ptr)
            .expect("rust_test_plugin should load successfully");
    }
}

// ---------------------------------------------------------------------------
// AC002 & AC004 & AC005 & AC007: Mismatch/zero is rejected with correct error
// ---------------------------------------------------------------------------

#[test]
fn test_mismatched_version_rejected() {
    let path = plugin_path("test_abi_mismatch");
    assert!(path.exists(), "Plugin .so not found at {path:?}");
    let mut api = make_engine_api();
    let world_ptr = make_world();
    let err = unsafe {
        engine_core::plugins::load_plugin(&path, &mut api, world_ptr)
            .expect_err("test_abi_mismatch should be rejected")
    };
    // AC007: Error message contains path, expected=1, actual=999
    assert!(
        err.contains("ABI version mismatch"),
        "Error should mention 'ABI version mismatch', got: {err}"
    );
    assert!(
        err.contains("expected 1"),
        "Error should contain 'expected 1', got: {err}"
    );
    assert!(
        err.contains("got 999"),
        "Error should contain 'got 999', got: {err}"
    );
    assert!(
        err.contains("test_abi_mismatch"),
        "Error should contain plugin name, got: {err}"
    );
}

#[test]
fn test_zero_version_rejected() {
    let path = plugin_path("test_abi_zero");
    assert!(path.exists(), "Plugin .so not found at {path:?}");
    let mut api = make_engine_api();
    let world_ptr = make_world();
    let err = unsafe {
        engine_core::plugins::load_plugin(&path, &mut api, world_ptr)
            .expect_err("test_abi_zero should be rejected")
    };
    assert!(err.contains("ABI version mismatch"), "Error: {err}");
    assert!(err.contains("expected 1"), "Error: {err}");
    assert!(err.contains("got 0"), "Error: {err}");
}

// ---------------------------------------------------------------------------
// AC008: All 5 loader functions perform the version check
// ---------------------------------------------------------------------------

#[test]
fn test_load_plugin_rejects_mismatch() {
    let path = plugin_path("test_abi_mismatch");
    assert!(path.exists());
    let mut api = make_engine_api();
    let world_ptr = make_world();
    let err = unsafe {
        engine_core::plugins::load_plugin(&path, &mut api, world_ptr)
            .expect_err("should be rejected")
    };
    assert!(err.contains("ABI version mismatch"), "{err}");
}

#[test]
fn test_load_plugin_and_register_worldgen_rejects_mismatch() {
    let path = plugin_path("test_abi_mismatch");
    assert!(path.exists());
    let mut api = make_engine_api();
    let world_ptr = make_world();
    let mut registry = engine_core::worldgen::WorldgenRegistry::new();
    let err = unsafe {
        engine_core::plugins::load_plugin_and_register_worldgen(
            &path,
            &mut api,
            world_ptr,
            &mut registry,
        )
        .expect_err("should be rejected")
    };
    assert!(err.contains("ABI version mismatch"), "{err}");
}

#[test]
fn test_load_plugin_and_register_worldgen_threadsafe_rejects_mismatch() {
    let path = plugin_path("test_abi_mismatch");
    assert!(path.exists());
    let mut api = make_engine_api();
    let world_ptr = make_world();
    let mut registry = engine_core::worldgen::ThreadSafeWorldgenRegistry::new();
    let err = unsafe {
        engine_core::plugins::load_plugin_and_register_worldgen_threadsafe(
            &path,
            &mut api,
            world_ptr,
            &mut registry,
        )
        .expect_err("should be rejected")
    };
    assert!(err.contains("ABI version mismatch"), "{err}");
}

#[test]
fn test_load_plugin_and_register_systems_rejects_mismatch() {
    let path = plugin_path("test_abi_mismatch");
    assert!(path.exists());
    let mut api = make_engine_api();
    let world_ptr = make_world();

    // Build a minimal World for system registration (even though init won't be reached)
    let mut registry = engine_core::ecs::registry::ComponentRegistry::new();
    let schema_json = r#"{
        "title": "Position",
        "type": "object",
        "properties": {
            "x": { "type": "number" },
            "y": { "type": "number" }
        },
        "required": ["x", "y"],
        "modes": ["colony", "roguelike"]
    }"#;
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));
    let mut world = engine_core::ecs::World::new(registry.clone());

    let err = unsafe {
        engine_core::plugins::load_plugin_and_register_systems(
            &path, &mut api, world_ptr, &mut world,
        )
        .expect_err("should be rejected")
    };
    assert!(err.contains("ABI version mismatch"), "{err}");
}

#[test]
fn test_load_plugin_with_manifest_rejects_mismatch() {
    let root = workspace_root();
    let plugin_dir = root.join("plugins").join("test_abi_mismatch");
    let manifest_path = plugin_dir.join("plugin.json");

    // Write a temporary manifest pointing to the mismatch plugin
    let manifest_content = r#"{
            "name": "ABI Mismatch Test",
            "version": "1.0.0",
            "description": "Testing ABI mismatch",
            "authors": ["MGE Test"],
            "dependencies": [],
            "dynamic_library": "libtest_abi_mismatch.so"
        }"#
    .to_string();
    std::fs::write(&manifest_path, &manifest_content).unwrap();

    let mut api = make_engine_api();
    let world_ptr = make_world();
    let err = unsafe {
        engine_core::plugins::load_plugin_with_manifest(&manifest_path, &mut api, world_ptr)
            .expect_err("should be rejected")
    };
    assert!(err.contains("ABI version mismatch"), "{err}");

    // Cleanup
    let _ = std::fs::remove_file(&manifest_path);
}
