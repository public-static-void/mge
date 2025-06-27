use engine_core::config::GameConfig;
use engine_core::plugins::loader::load_native_plugins_from_config_threadsafe;
use engine_core::plugins::types::EngineApi;
use engine_core::worldgen::GLOBAL_WORLDGEN_REGISTRY;
use std::path::PathBuf;

fn find_workspace_root() -> PathBuf {
    if let Ok(root) = std::env::var("MGE_WORKSPACE_ROOT") {
        return PathBuf::from(root);
    }
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("game.toml").exists() {
            return dir;
        }
        if !dir.pop() {
            panic!("Could not find workspace root containing game.toml");
        }
    }
}

fn find_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("MGE_CONFIG_FILE") {
        PathBuf::from(path)
    } else {
        find_workspace_root().join("game.toml")
    }
}

pub fn register_plugins() {
    let config_path = find_config_path();
    let config = GameConfig::load_from_file(&config_path).unwrap_or_else(|_| {
        panic!(
            "Failed to load config for plugin registration: {config_path:?}"
        )
    });

    let mut engine_api = EngineApi {
        spawn_entity: engine_core::plugins::ffi::ffi_spawn_entity,
        set_component: engine_core::plugins::ffi::ffi_set_component,
    };
    let world_ptr: *mut std::os::raw::c_void = std::ptr::null_mut();

    let mut registry = GLOBAL_WORLDGEN_REGISTRY.lock().unwrap();
    unsafe {
        load_native_plugins_from_config_threadsafe(
            &config,
            &mut engine_api,
            world_ptr,
            &mut registry, // <-- fixed here
        )
    }
    .expect("Failed to load native plugins from config at engine_py init");
}
