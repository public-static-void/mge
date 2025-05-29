use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::mods::loader::load_mod;
use engine_core::scripting::ScriptEngine;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
        let mod_dir = format!("mods/{}", mod_name);

        // Load engine schemas first
        let engine_schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
        let engine_schemas = engine_core::ecs::schema::load_schemas_from_dir(&engine_schema_dir)
            .expect("Failed to load engine schemas");

        let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
        for (_name, schema) in engine_schemas {
            registry.lock().unwrap().register_external_schema(schema);
        }

        // Read mod manifest and parse mode if present
        let manifest_path = format!("{}/mod.json", mod_dir);
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
            eprintln!("Failed to load mod: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // --- Script/demo/test mode ---
    if let Some(script_path) = script_path {
        let script = fs::read_to_string(&script_path).unwrap_or_else(|_| {
            eprintln!("Failed to read Lua script file: {}", script_path);
            std::process::exit(1);
        });

        let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
        let schemas = engine_core::ecs::schema::load_schemas_from_dir(&schema_dir)
            .expect("Failed to load schemas");
        let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
        for (_name, schema) in schemas {
            registry.lock().unwrap().register_external_schema(schema);
        }

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
            eprintln!("Lua error: {:?}", e);
            std::process::exit(1);
        }
        return;
    }

    eprintln!("Usage: mge-cli --mod <mod_name> [--mode <mode>] | <script.lua> [args...]");
    std::process::exit(1);
}
