// ─── ABI version tests ───
//! Verifies:
//! - AC001: Matching version loads successfully
//! - AC002: Mismatched version is rejected
//! - AC003: Version check precedes init (structural, verified by test flow)
//! - AC004: Error is returned, not panicked
//! - AC005: Pre-versioning plugin (abi_version=0) is rejected
//! - AC007: Error message contains path, expected, actual
//! - AC008: All 5 loader functions perform the check

use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::Value as JsonValue;
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

    let _ = std::fs::remove_file(&manifest_path);
}

// ─── WASM world reservation tests ───

#[test]
fn test_wasm_world_reservation_flow() {
    let mut world = WasmWorld::new();

    let stockpile_eid = world.spawn_entity();
    let stockpile_data = serde_json::json!({"resources": {"iron_ore": 100.0}});
    world
        .set_component(
            stockpile_eid,
            "Stockpile",
            &serde_json::to_string(&stockpile_data).unwrap(),
        )
        .unwrap();

    let stockpile_str = world.get_component(stockpile_eid, "Stockpile").unwrap();
    let stockpile_val: JsonValue = serde_json::from_str(&stockpile_str).unwrap();
    assert_eq!(
        stockpile_val["resources"]["iron_ore"], 100.0,
        "Stockpile should have iron_ore 100.0"
    );

    let job_eid = world.spawn_entity();
    let job_data = serde_json::json!({
        "state": "pending",
        "resource_requirements": [{"kind": "iron_ore", "amount": 10}]
    });
    world
        .set_component(job_eid, "Job", &serde_json::to_string(&job_data).unwrap())
        .unwrap();

    let job_str = world.get_component(job_eid, "Job").unwrap();
    let job_val: JsonValue = serde_json::from_str(&job_str).unwrap();
    assert_eq!(job_val["state"], "pending", "Job state should be pending");
    assert!(
        job_val.get("resource_requirements").is_some(),
        "Job should have resource_requirements"
    );

    let stockpile_entities = world.get_entities_with_component("Stockpile");
    assert_eq!(
        stockpile_entities,
        vec![stockpile_eid],
        "Should find stockpile entity"
    );
    let job_entities = world.get_entities_with_component("Job");
    assert_eq!(job_entities, vec![job_eid], "Should find job entity");

    let reservations = world.get_job_resource_reservations(job_eid);
    assert!(
        reservations.is_none(),
        "Should have no reservations before reserve"
    );

    world.reserve_job_resources();

    let reservations = world.get_job_resource_reservations(job_eid);
    assert!(
        reservations.is_some(),
        "Should have reservations after reserve"
    );
    let reservations_str = reservations.unwrap();
    let res_value: JsonValue = serde_json::from_str(&reservations_str).unwrap();
    assert!(
        res_value.is_array(),
        "Reserved resources should be an array"
    );
    assert!(
        !res_value.as_array().unwrap().is_empty(),
        "Reserved resources should not be empty"
    );

    world.release_job_resource_reservations(job_eid);

    let reservations = world.get_job_resource_reservations(job_eid);
    assert!(
        reservations.is_none(),
        "Should have no reservations after release"
    );
}

#[test]
fn test_wasm_world_reservation_insufficient_resources() {
    let mut world = WasmWorld::new();

    let stockpile_eid = world.spawn_entity();
    let stockpile_data = serde_json::json!({"resources": {"iron_ore": 5.0}});
    world
        .set_component(
            stockpile_eid,
            "Stockpile",
            &serde_json::to_string(&stockpile_data).unwrap(),
        )
        .unwrap();

    let job_eid = world.spawn_entity();
    let job_data = serde_json::json!({
        "state": "pending",
        "resource_requirements": [{"kind": "iron_ore", "amount": 10}]
    });
    world
        .set_component(job_eid, "Job", &serde_json::to_string(&job_data).unwrap())
        .unwrap();

    world.reserve_job_resources();

    let reservations = world.get_job_resource_reservations(job_eid);
    assert!(
        reservations.is_none(),
        "Should have no reservations with insufficient resources"
    );
}
