use engine_core::ecs::registry::ComponentRegistry;
use engine_core::scripting::{ScriptEngine, World};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::rc::Rc;
use std::sync::Arc;

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

    // Load all schemas!
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = engine_core::ecs::schema::load_schemas_from_dir(&schema_dir)
        .expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(registry);

    let world = Rc::new(RefCell::new(World::new(registry.clone())));
    let mut engine = ScriptEngine::new();
    engine
        .register_world(world.clone())
        .expect("Failed to register ECS API");

    // --- Run script ---
    if let Err(e) = engine.run_script(&script) {
        eprintln!("Lua error: {e}");
        std::process::exit(1);
    }
}
