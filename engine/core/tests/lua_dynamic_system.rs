use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::scripting::engine::ScriptEngine;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn test_lua_dynamic_system_registration_and_run() {
    let mut engine = ScriptEngine::new();

    // Create a registry and world
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world = Rc::new(RefCell::new(World::new(registry.clone())));

    engine.register_world(world.clone()).unwrap();

    // Load and run the Lua test script
    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // go up from engine/core to engine/
        .join("scripts/lua/test_dynamic_system.lua");

    println!("Looking for Lua script at: {:?}", script_path);

    assert!(
        script_path.exists(),
        "Lua script not found at {:?}",
        script_path
    );

    let code = std::fs::read_to_string(&script_path).unwrap();
    assert!(engine.run_script(&code).is_ok());
}
