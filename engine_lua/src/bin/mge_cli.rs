use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::mods::loader::load_mod;
use engine_core::plugins::loader::load_native_plugins_from_config;
use engine_core::plugins::types::EngineApi;
use engine_core::worldgen::WorldgenRegistry;
use engine_lua::ScriptEngine;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Returns the absolute path to the engine's schema directory,
/// robust to workspace layout and usable for both dev and test.
fn find_schema_dir() -> PathBuf {
    // Try environment variable override first (for CI/tests)
    if let Ok(dir) = env::var("MGE_SCHEMA_DIR") {
        return PathBuf::from(dir);
    }
    // Default: relative to engine_lua's Cargo.toml
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/assets/schemas")
}

fn find_config_file() -> PathBuf {
    // Try env var override first
    if let Ok(path) = env::var("MGE_CONFIG_FILE") {
        return PathBuf::from(path);
    }
    // Default: project root
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game.toml")
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut mode_arg: Option<String> = None;
    let mut mod_name: Option<String> = None;
    let mut script_path: Option<String> = None;
    let mut script_args: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mod" if i + 1 < args.len() => {
                mod_name = Some(args[i + 1].clone());
                i += 2;
            }
            "--mode" if i + 1 < args.len() => {
                mode_arg = Some(args[i + 1].clone());
                i += 2;
            }
            _ if script_path.is_none() => {
                script_path = Some(args[i].clone());
                i += 1;
            }
            _ => {
                script_args.push(args[i].clone());
                i += 1;
            }
        }
    }

    // --- Mod loading mode ---
    if let Some(mod_name) = mod_name {
        let mod_dir = format!("mods/{mod_name}");

        // Load engine schemas first
        let schema_dir = find_schema_dir();
        if !schema_dir.exists() {
            eprintln!(
                "Schema directory does not exist: {schema_dir:?}\n\
                Set MGE_SCHEMA_DIR or check your workspace structure."
            );
            std::process::exit(1);
        }
        let config_file = find_config_file();
        let config = GameConfig::load_from_file(&config_file)
            .unwrap_or_else(|e| panic!("Failed to load config from {config_file:?}: {e:?}"));
        let engine_schemas = engine_core::ecs::schema::load_schemas_from_dir_with_modes(
            &schema_dir,
            &config.allowed_modes,
        )
        .expect("Failed to load engine schemas");

        let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
        for (_name, schema) in engine_schemas {
            registry.lock().unwrap().register_external_schema(schema);
        }

        // --- Plugin registration ---
        let mut engine_api = EngineApi {
            // Fill with function pointers as needed for your engine
            spawn_entity: engine_core::plugins::ffi::ffi_spawn_entity,
            set_component: engine_core::plugins::ffi::ffi_set_component,
        };
        let mut worldgen_registry = WorldgenRegistry::new();
        let world_ptr: *mut std::os::raw::c_void = std::ptr::null_mut(); // Use actual pointer if needed

        unsafe {
            load_native_plugins_from_config(
                &config,
                &mut engine_api,
                world_ptr,
                &mut worldgen_registry,
            )
        }
        .expect("Failed to load native plugins from config");

        // Read mod manifest and parse mode if present
        let manifest_path = format!("{mod_dir}/mod.json");
        let manifest: Option<serde_json::Value> = fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok());

        let manifest_mode = manifest
            .as_ref()
            .and_then(|m| m.get("mode"))
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        let mode = mode_arg
            .or(manifest_mode)
            .unwrap_or_else(|| "colony".to_string());

        let mut world = World::new(registry.clone());
        world.current_mode = mode.clone();

        let world_rc = Rc::new(RefCell::new(world));
        let mut engine = ScriptEngine::new();
        engine
            .register_world(world_rc.clone())
            .expect("Failed to register ECS API");

        if let Err(e) = load_mod(&mod_dir, world_rc.clone(), &mut engine) {
            eprintln!("Failed to load mod: {e}");
            std::process::exit(1);
        }
        return;
    }

    // --- Script/demo/test mode ---
    if let Some(script_path) = script_path {
        let script = fs::read_to_string(&script_path).unwrap_or_else(|_| {
            eprintln!("Failed to read Lua script file: {script_path}");
            std::process::exit(1);
        });

        let schema_dir = find_schema_dir();
        if !schema_dir.exists() {
            eprintln!(
                "Schema directory does not exist: {schema_dir:?}\n\
                Set MGE_SCHEMA_DIR or check your workspace structure."
            );
            std::process::exit(1);
        }
        let config_file = find_config_file();
        let config = GameConfig::load_from_file(&config_file)
            .unwrap_or_else(|e| panic!("Failed to load config from {config_file:?}: {e:?}"));
        let schemas = engine_core::ecs::schema::load_schemas_from_dir_with_modes(
            &schema_dir,
            &config.allowed_modes,
        )
        .expect("Failed to load schemas");
        let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
        for (_name, schema) in schemas {
            registry.lock().unwrap().register_external_schema(schema);
        }

        // --- Plugin registration ---
        let mut engine_api = EngineApi {
            spawn_entity: engine_core::plugins::ffi::ffi_spawn_entity,
            set_component: engine_core::plugins::ffi::ffi_set_component,
        };
        let mut worldgen_registry = WorldgenRegistry::new();
        let world_ptr: *mut std::os::raw::c_void = std::ptr::null_mut();

        unsafe {
            load_native_plugins_from_config(
                &config,
                &mut engine_api,
                world_ptr,
                &mut worldgen_registry,
            )
        }
        .expect("Failed to load native plugins from config");

        let mut world = World::new(registry.clone());
        if let Some(mode) = mode_arg {
            world.current_mode = mode;
        }

        let world_rc = Rc::new(RefCell::new(world));
        let mut engine = ScriptEngine::new();
        engine
            .register_world(world_rc.clone())
            .expect("Failed to register ECS API");
        engine.set_lua_args(script_args);

        if let Err(e) = engine.run_script(&script) {
            eprintln!("Lua error: {e:?}");
            std::process::exit(1);
        }
        return;
    }

    eprintln!("Usage: mge-cli --mod <mod_name> [--mode <mode>] | <script.lua> [args...]");
    std::process::exit(1);
}
