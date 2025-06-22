// engine/core/tests/helpers/worldgen.rs
use engine_core::plugins::loader::load_plugin_and_register_worldgen;
use engine_core::plugins::types::EngineApi;
use engine_core::worldgen::WorldgenRegistry;
use std::os::raw::{c_char, c_void};
use std::path::PathBuf;

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

pub fn setup_registry_with_c_plugin() -> WorldgenRegistry {
    let mut registry = WorldgenRegistry::new();
    let mut engine_api = EngineApi {
        spawn_entity: test_spawn_entity,
        set_component: test_set_component,
    };
    let world_ptr = std::ptr::null_mut();

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
