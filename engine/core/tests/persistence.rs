#[path = "helpers/world.rs"]
mod world_helper;

#[path = "helpers/world_io.rs"]
mod world_io_helper;

use engine_core::ecs::components::position::{Position, PositionComponent};
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::mods::loader::{ModScriptEngine, load_mod};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use world_helper::make_test_world;
use world_io_helper::save_and_load_roundtrip;

// === save_load tests ===

#[test]
fn test_save_and_load_world_roundtrip() {
    let world = make_test_world();
    let registry = world.registry.clone();

    let mut world = world;
    world.current_mode = "roguelike".to_string();

    let e1 = world.spawn_entity();
    world
        .set_component(
            e1,
            "Health",
            serde_json::json!({ "current": 42, "max": 100 }),
        )
        .unwrap();

    let e2 = world.spawn_entity();
    let pos = PositionComponent {
        pos: Position::Square { x: 1, y: 2, z: 0 },
    };
    world
        .set_component(e2, "Position", serde_json::to_value(&pos).unwrap())
        .unwrap();

    let loaded_world = save_and_load_roundtrip(&world, registry);

    assert_eq!(world.entities, loaded_world.entities);
    assert_eq!(
        world.get_component(e1, "Health"),
        loaded_world.get_component(e1, "Health")
    );
    assert_eq!(
        world.get_component(e2, "Position"),
        loaded_world.get_component(e2, "Position")
    );
}

// === mod_loader tests ===

pub fn setup_test_mod_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp_dir = tempfile::tempdir().unwrap();
    let mod_dir = temp_dir.path().join("example_mod");
    std::fs::create_dir(&mod_dir).unwrap();
    std::fs::create_dir(mod_dir.join("schemas")).unwrap();
    std::fs::create_dir(mod_dir.join("systems")).unwrap();

    let schema = r#"{
        "title": "TestComponent",
        "type": "object",
        "properties": { "foo": { "type": "number" } },
        "required": ["foo"],
        "modes": ["colony"]
    }"#;
    std::fs::write(mod_dir.join("schemas").join("test_component.json"), schema).unwrap();

    let system = r#""#;
    std::fs::write(mod_dir.join("systems").join("test_system.txt"), system).unwrap();

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

    assert!(
        registry
            .lock()
            .unwrap()
            .get_schema_by_name("TestComponent")
            .is_some()
    );

    assert!(
        world_rc
            .borrow()
            .list_systems()
            .contains(&"TestSystem".to_string())
    );
}
