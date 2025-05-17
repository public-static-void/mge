use engine_core::ecs::registry::ComponentRegistry;
use engine_core::mods::loader::load_mod;
use engine_core::scripting::ScriptEngine;
use engine_core::scripting::world::World;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn setup_test_mod_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp_dir = tempfile::tempdir().unwrap();
    let mod_dir = temp_dir.path().join("example_mod");
    std::fs::create_dir(&mod_dir).unwrap();
    std::fs::create_dir(mod_dir.join("schemas")).unwrap();
    std::fs::create_dir(mod_dir.join("systems")).unwrap();

    // Write schema
    let schema = r#"{
        "name": "TestComponent",
        "schema": {
            "title": "TestComponent",
            "type": "object",
            "properties": { "foo": { "type": "number" } },
            "required": ["foo"],
            "modes": ["colony"]
        },
        "modes": ["colony"]
    }"#;
    std::fs::write(mod_dir.join("schemas").join("test_component.json"), schema).unwrap();

    // Write dummy Lua system
    let lua_system = r#"register_system("TestSystem", function() end)"#;
    std::fs::write(mod_dir.join("systems").join("test_system.lua"), lua_system).unwrap();

    // Write mod.json
    let manifest = r#"{
        "name": "example_mod",
        "version": "1.0.0",
        "schemas": ["schemas/test_component.json"],
        "systems": [
            { "file": "systems/test_system.lua", "name": "TestSystem" }
        ]
    }"#;
    std::fs::write(mod_dir.join("mod.json"), manifest).unwrap();

    (temp_dir, mod_dir)
}

#[test]
fn test_load_mod_registers_schema_and_system() {
    let (_temp_dir, mod_dir) = setup_test_mod_dir();
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world = World::new(registry.clone());
    let mut script_engine = ScriptEngine::new();
    let world_rc = Rc::new(RefCell::new(world));
    script_engine.register_world(world_rc.clone()).unwrap();

    load_mod(&mod_dir, world_rc.clone(), &mut script_engine).expect("Mod should load");

    // Assert schema is registered
    assert!(
        registry
            .lock()
            .unwrap()
            .get_schema_by_name("TestComponent")
            .is_some()
    );

    // Assert system is registered (depends on your API)
    assert!(
        world_rc
            .borrow()
            .list_systems()
            .contains(&"TestSystem".to_string())
    );
}
