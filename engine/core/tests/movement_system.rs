use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::scripting::ScriptEngine;
use engine_core::systems::standard::MoveAll;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_move_all_moves_positions() {
    // Load schemas
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    let id1 = world.spawn_entity();
    let id2 = world.spawn_entity();

    // Set initial positions
    world
        .set_component(id1, "Position", json!({ "x": 1.0, "y": 2.0 }))
        .unwrap();
    world
        .set_component(id2, "Position", json!({ "x": 5.0, "y": 7.0 }))
        .unwrap();

    // Call move_all (to be implemented)
    world.register_system(MoveAll { dx: 1.0, dy: -1.0 });
    world.run_system("MoveAll", None).unwrap();

    // Check new positions
    let pos1 = world.get_component(id1, "Position").unwrap();
    let pos2 = world.get_component(id2, "Position").unwrap();

    assert!((pos1["x"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    assert!((pos1["y"].as_f64().unwrap() - 1.0).abs() < 1e-6);
    assert!((pos2["x"].as_f64().unwrap() - 6.0).abs() < 1e-6);
    assert!((pos2["y"].as_f64().unwrap() - 6.0).abs() < 1e-6);
}

#[test]
fn test_lua_move_all() {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));

    let mut engine = ScriptEngine::new();
    let world = std::rc::Rc::new(std::cell::RefCell::new(World::new(registry.clone())));
    world.borrow_mut().current_mode = "colony".to_string();
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local id = spawn_entity()
        set_component(id, "Position", { x = 0.0, y = 0.0 })
        move_all(2.0, 3.0)
        local pos = get_component(id, "Position")
        assert(math.abs(pos.x - 2.0) < 1e-6)
        assert(math.abs(pos.y - 3.0) < 1e-6)
    "#;

    engine.run_script(script).unwrap();
}
