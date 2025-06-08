use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::mods::loader::{ModScriptEngine, load_mod};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// Creates a dummy mod directory with a schema, a system, and a manifest.
/// Returns (TempDir, mod_dir_path).
pub fn setup_test_mod_dir() -> (TempDir, std::path::PathBuf) {
    let temp_dir = tempfile::tempdir().unwrap();
    let mod_dir = temp_dir.path().join("example_mod");
    std::fs::create_dir(&mod_dir).unwrap();
    std::fs::create_dir(mod_dir.join("schemas")).unwrap();
    std::fs::create_dir(mod_dir.join("systems")).unwrap();

    // Write schema
    let schema = r#"{
        "title": "TestComponent",
        "type": "object",
        "properties": { "foo": { "type": "number" } },
        "required": ["foo"],
        "modes": ["colony"]
    }"#;
    std::fs::write(mod_dir.join("schemas").join("test_component.json"), schema).unwrap();

    // Write dummy system (contents are backend-specific)
    let system = r#""#;
    std::fs::write(mod_dir.join("systems").join("test_system.txt"), system).unwrap();

    // Write mod.json
    let manifest = r#"{
        "name": "example_mod",
        "version": "1.0.0",
        "schemas": ["schemas/test_component.json"],
        "systems": [
            { "file": "systems/test_system.txt", "name": "TestSystem" }
        ],
        "main_script": "systems/test_system.txt"
    }"#;
    std::fs::write(mod_dir.join("mod.json"), manifest).unwrap();

    (temp_dir, mod_dir)
}

/// Generic test for mod loading. Backend-specific tests should call this with their ScriptEngine.
pub fn mod_loader_test<E: ModScriptEngine + Default>(mut script_engine: E) {
    let (_temp_dir, mod_dir) = setup_test_mod_dir();
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world = World::new(registry.clone());
    let world_rc = Rc::new(RefCell::new(world));

    load_mod(
        mod_dir.to_str().unwrap(),
        world_rc.clone(),
        &mut script_engine,
    )
    .expect("Mod should load");

    // Assert schema is registered
    assert!(
        registry
            .lock()
            .unwrap()
            .get_schema_by_name("TestComponent")
            .is_some()
    );

    // Assert system is registered (depends on API)
    assert!(
        world_rc
            .borrow()
            .list_systems()
            .contains(&"TestSystem".to_string())
    );
}
