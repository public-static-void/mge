use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::map::{Map, SquareGridMap};
use engine_core::scripting::ScriptEngine;
use engine_core::systems::body_equipment_sync::BodyEquipmentSyncSystem;
use engine_core::systems::economic::{EconomicSystem, load_recipes_from_dir};
use engine_core::systems::equipment_logic::EquipmentLogicSystem;
use engine_core::systems::inventory::InventoryConstraintSystem;
use engine_core::systems::job::{JobSystem, JobTypeRegistry, load_job_types_from_dir};
use engine_core::systems::standard::{DamageAll, MoveAll, MoveDelta, ProcessDeaths, ProcessDecay};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: mge-cli <script.lua> [args...]");
        std::process::exit(1);
    }

    let script_path = &args[1];
    let script = fs::read_to_string(script_path).unwrap_or_else(|_| {
        eprintln!("Failed to read Lua script file: {}", script_path);
        std::process::exit(1);
    });

    let lua_args = args[2..].to_vec();

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

    // Create the square grid map and add the required cells
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 2, 0); // starting cell for your test entity
    grid.add_cell(1, 2, 0); // cell you want to move to

    // Wrap in Map and assign to world
    let map = Map::new(Box::new(grid));
    world.borrow_mut().map = Some(map);

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

    // --- Economic System registration (NEW) ---
    let recipes = load_recipes_from_dir("engine/assets/recipes");
    let economic_system = EconomicSystem::with_recipes(recipes);
    world.borrow_mut().register_system(economic_system);

    // --- Job System registration ---
    let job_types = load_job_types_from_dir("assets/jobs");
    let mut job_registry = JobTypeRegistry::default();
    for job in job_types {
        job_registry.register_data_job(job);
    }
    let job_system = JobSystem::with_registry(job_registry);
    world.borrow_mut().register_system(job_system);

    world
        .borrow_mut()
        .register_system(InventoryConstraintSystem);
    world.borrow_mut().register_system(EquipmentLogicSystem);
    world.borrow_mut().register_system(BodyEquipmentSyncSystem);

    let mut engine = ScriptEngine::new();
    engine
        .register_world(world.clone())
        .expect("Failed to register ECS API");

    // --- Pass arguments to Lua ---
    engine.set_lua_args(lua_args);

    // --- Run script ---
    if let Err(e) = engine.run_script(&script) {
        eprintln!("Lua error: {:?}", e);
        std::process::exit(1);
    }
}
