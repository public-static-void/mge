use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::map::{Map, SquareGridMap};
use engine_core::scripting::ScriptEngine;
use engine_core::systems::body_equipment_sync::BodyEquipmentSyncSystem;
use engine_core::systems::death_decay::{ProcessDeaths, ProcessDecay};
use engine_core::systems::economic::{EconomicSystem, load_recipes_from_dir};
use engine_core::systems::equipment_logic::EquipmentLogicSystem;
use engine_core::systems::inventory::InventoryConstraintSystem;
use engine_core::systems::job::{JobSystem, JobTypeRegistry, load_job_types_from_dir};
use gag::BufferRedirect;
use regex::Regex;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

const COLOR_RESET: &str = "\x1b[0m";
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_RED: &str = "\x1b[31m";
const COLOR_CYAN: &str = "\x1b[36m";

fn color(c: &str, s: &str) -> String {
    format!("{}{}{}", c, s, COLOR_RESET)
}

fn indent_lines(s: &str) -> String {
    s.lines()
        .map(|l| format!("  {}", l))
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let filter_module = args.first().map(|s| s.as_str());
    let filter_func = args.get(1).map(|s| s.as_str());

    // Compile the regex ONCE, outside the loop (Clippy requirement)
    let re = Regex::new(r#"test_[a-zA-Z0-9_]+\s*="#).unwrap();

    // Discover test functions, filtered by module and/or function if provided
    let test_dir = Path::new("engine/scripts/lua/tests");
    let mut test_functions = Vec::new();

    for entry in fs::read_dir(test_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
            if fname.starts_with("test_") && fname.ends_with(".lua") {
                let modname = &fname[..fname.len() - 4];
                if filter_module.is_none_or(|f| modname == f) {
                    let content = fs::read_to_string(&path)?;
                    // Use the regex compiled above
                    for cap in re.find_iter(&content) {
                        let key = cap.as_str().split('=').next().unwrap().trim();
                        // Apply filters as before
                        if let (Some(fmod), Some(ffunc)) = (filter_module, filter_func) {
                            if modname == fmod && key == ffunc {
                                test_functions.push((modname.to_string(), key.to_string()));
                            }
                        } else if let (None, Some(_)) = (filter_module, filter_func) {
                            // Do nothing: function filter without module filter is not supported
                        } else if let (Some(fmod), None) = (filter_module, filter_func) {
                            if modname == fmod {
                                test_functions.push((modname.to_string(), key.to_string()));
                            }
                        } else if filter_module.is_none() && filter_func.is_none() {
                            test_functions.push((modname.to_string(), key.to_string()));
                        }
                    }
                }
            }
        }
    }

    if test_functions.is_empty() {
        eprintln!("No tests found matching your filters.");
        std::process::exit(1);
    }

    // Run each test in a fresh World and Lua state
    let mut total = 0;
    let mut failed = 0;
    let mut failed_tests = Vec::new();

    for (i, (modname, fname)) in test_functions.iter().enumerate() {
        if i > 0 {
            println!(
                "{}",
                color(
                    COLOR_CYAN,
                    "--------------------------------------------------"
                )
            );
        }

        total += 1;
        let testname = format!("{}.{}", modname, fname);

        // Print the RUN line before any test output
        println!("{}{}", color(COLOR_CYAN, "[RUN]    "), testname);

        // --- ECS + Lua context ---
        let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
        let schemas = load_schemas_from_dir(&schema_dir)?;
        let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
        for (_name, schema) in schemas.clone() {
            registry.lock().unwrap().register_external_schema(schema);
        }

        let world = Rc::new(RefCell::new(World::new(registry.clone())));

        let mut grid = SquareGridMap::new();
        grid.add_cell(0, 2, 0);
        grid.add_cell(1, 2, 0);
        let map = Map::new(Box::new(grid));
        world.borrow_mut().map = Some(map);

        // Move all: increment x for all entities with Position
        if let Some(positions) = world.borrow_mut().components.get_mut("Position") {
            for (_eid, value) in positions.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(x) = obj.get_mut("x") {
                        if let Some(x_val) = x.as_f64() {
                            *x = serde_json::json!(x_val + 1.0);
                        }
                    }
                }
            }
        }
        // Damage all: decrement health for all entities with Health
        if let Some(healths) = world.borrow_mut().components.get_mut("Health") {
            for (_eid, value) in healths.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(current) = obj.get_mut("current") {
                        if let Some(cur_val) = current.as_f64() {
                            let new_val = (cur_val - 1.0).max(0.0);
                            *current = serde_json::json!(new_val);
                        }
                    }
                }
            }
        }
        world.borrow_mut().register_system(ProcessDeaths);
        world.borrow_mut().register_system(ProcessDecay);

        // --- Economic System registration ---
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

        // --- Set Lua package.path to include the tests directory ---
        let lua = Rc::clone(&engine.lua);
        let package: mlua::Table = lua.globals().get("package")?;
        let old_path: String = package.get("path")?;
        let new_path = format!("engine/scripts/lua/tests/?.lua;{}", old_path);
        package.set("path", new_path)?;

        // Prepare Lua code: require the module and call only the test function
        let script = format!(
            r#"
            local mod = require("{mod}");
            mod["{func}"]();
            "#,
            mod = modname,
            func = fname
        );

        // --- Capture stdout/stderr for this test only during test execution ---
        let out_buf = BufferRedirect::stdout().unwrap();
        let err_buf = BufferRedirect::stderr().unwrap();

        // Run the test
        let result = engine.run_script(&script);

        // Stop capturing and read output
        drop(out_buf);
        drop(err_buf);

        let mut output = String::new();
        let mut error_output = String::new();

        // Re-capture from the buffers (they must be dropped first for all output to flush)
        let mut out_buf = BufferRedirect::stdout().unwrap();
        let mut err_buf = BufferRedirect::stderr().unwrap();
        out_buf.read_to_string(&mut output).ok();
        err_buf.read_to_string(&mut error_output).ok();
        drop(out_buf);
        drop(err_buf);

        // Print captured output, indented
        if !output.trim().is_empty() {
            print!("{}", indent_lines(&output));
        }
        if !error_output.trim().is_empty() {
            print!("{}", indent_lines(&error_output));
        }

        // Print status line after all test output, always aligned
        match result {
            Ok(_) => {
                println!("{} {}", color(COLOR_GREEN, "[OK]    "), testname);
            }
            Err(e) => {
                println!("{} {}", color(COLOR_RED, "[FAIL]  "), testname);
                let err_str = match &e {
                    mlua::Error::RuntimeError(msg) => msg.clone(),
                    mlua::Error::SyntaxError { message, .. } => message.clone(),
                    _ => format!("{:?}", e),
                };
                // Print error and stacktrace indented
                println!("{}", indent_lines(&err_str));
                failed += 1;
                failed_tests.push((testname, err_str));
            }
        }
    }

    println!("{}", color(COLOR_CYAN, &"-".repeat(60)));
    println!("{} tests run, {} failed", total, failed);

    if failed > 0 {
        println!("{}", color(COLOR_RED, "\nFailed tests:"));
        for (name, err) in &failed_tests {
            println!("{}", color(COLOR_RED, name));
            println!("{}", indent_lines(err));
        }
        std::process::exit(1);
    } else {
        println!("{}", color(COLOR_GREEN, "All tests passed!"));
        std::process::exit(0);
    }
}
