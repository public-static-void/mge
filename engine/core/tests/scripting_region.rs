use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::scripting::ScriptEngine;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_region_lua_api() {
    let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas");
    let schemas = load_schemas_from_dir(schema_dir).unwrap();
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(std::sync::Mutex::new(registry));
    let world = World::new(registry);
    let mut script_engine = ScriptEngine::new();
    script_engine
        .register_world(std::rc::Rc::new(RefCell::new(world)))
        .unwrap();

    // Robust absolute path
    let lua_script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // engine/
        .parent()
        .unwrap() // project root
        .join("engine/scripts/lua/tests/scripting_region.lua");
    println!("Lua script path: {:?}", lua_script_path);
    let lua_script = std::fs::read_to_string(&lua_script_path).unwrap();
    script_engine.run_script(&lua_script).unwrap();
}
