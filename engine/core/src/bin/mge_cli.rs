use engine_core::scripting::{ScriptEngine, World};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::rc::Rc;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: mge-cli <script.lua>");
        std::process::exit(1);
    }

    let script_path = &args[1];
    let script = fs::read_to_string(script_path).unwrap_or_else(|_| {
        eprintln!("Failed to read Lua script file: {}", script_path);
        std::process::exit(1);
    });

    // --- ECS + Lua context ---
    let world = Rc::new(RefCell::new(World::new()));
    let engine = ScriptEngine::new();
    engine
        .register_world(world.clone())
        .expect("Failed to register ECS API");

    // Optionally, override print here if you want to capture output
    // (Or add a print override to ScriptEngine in the future)

    // --- Run script ---
    if let Err(e) = engine.run_script(&script) {
        eprintln!("Lua error: {e}");
        std::process::exit(1);
    }
}
