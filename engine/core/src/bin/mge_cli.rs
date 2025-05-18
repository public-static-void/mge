use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::scripting::ScriptEngine;
use engine_core::systems::job::{JobSystem, JobTypeRegistry, load_job_types_from_dir};
use engine_core::systems::standard::{DamageAll, MoveAll, MoveDelta, ProcessDeaths, ProcessDecay};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    for (_name, schema) in schemas {
        registry.lock().unwrap().register_external_schema(schema);
    }

    let world = Rc::new(RefCell::new(World::new(registry.clone())));
    world.borrow_mut().register_system(MoveAll {
        delta: MoveDelta::Square {
            dx: 1,
            dy: 0,
            dz: 0,
        },
    });
    world
        .borrow_mut()
        .register_system(DamageAll { amount: 1.0 });
    world.borrow_mut().register_system(ProcessDeaths);
    world.borrow_mut().register_system(ProcessDecay);

    let job_types = load_job_types_from_dir("assets/jobs");
    let mut job_registry = JobTypeRegistry::default();
    for job in job_types {
        job_registry.register_data_job(job);
    }
    let job_system = JobSystem::with_registry(job_registry);
    world.borrow_mut().register_system(job_system);

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
