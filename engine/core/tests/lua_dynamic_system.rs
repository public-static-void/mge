use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::scripting::engine::ScriptEngine;
use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn test_lua_dynamic_system_registration_and_run() {
    // Set LUA_PATH so Lua can find luaunit.lua in the tests directory
    let lua_test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("scripts/lua/tests");
    let lua_path = format!("{}/?.lua;;", lua_test_dir.display());
    unsafe {
        env::set_var("LUA_PATH", &lua_path);
    }

    let mut engine = ScriptEngine::new();

    // Create a registry and world
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world = Rc::new(RefCell::new(World::new(registry.clone())));

    engine.register_world(world.clone()).unwrap();

    // Load and run the Lua test script
    let script_path = lua_test_dir.join("test_dynamic_system.lua");

    println!("Looking for Lua script at: {:?}", script_path);

    assert!(
        script_path.exists(),
        "Lua script not found at {:?}",
        script_path
    );

    let code = std::fs::read_to_string(&script_path).unwrap();
    if let Err(e) = engine.run_script(&code) {
        panic!("Lua script execution failed: {:?}", e);
    }
}
