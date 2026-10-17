//! Integration test for the native planet-biome worldgen plugin.
//!
//! Verifies the plugin `.so` loads through the worldgen loader and registers
//! under a name distinct from the builtin, so a builtin-populated registry
//! never rejects it as a duplicate.

use engine_core::ecs::{ComponentRegistry, World};
use engine_core::plugins::{EngineApi, load_plugin_and_register_worldgen};
use engine_core::worldgen::WorldgenRegistry;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Registry name the plugin declares through `worldgen_name`.
const PLUGIN_NAME: &str = "planet_biomes_native";
/// Builtin name the plugin must not collide with.
const BUILTIN_NAME: &str = "planet_biomes";

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
