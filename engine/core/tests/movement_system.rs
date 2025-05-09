use engine_core::ecs::registry::ComponentRegistry;
use engine_core::scripting::{ScriptEngine, World};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[test]
fn test_move_all_moves_positions() {
    let registry = Arc::new(ComponentRegistry::new());
    let mut world = World::new(registry.clone());

    let id1 = world.spawn();
    let id2 = world.spawn();

    // Set initial positions
    world
        .set_component(id1, "Position", serde_json::json!({ "x": 1.0, "y": 2.0 }))
        .unwrap();
    world
        .set_component(id2, "Position", serde_json::json!({ "x": 5.0, "y": 7.0 }))
        .unwrap();

    // Call move_all (to be implemented)
    world.move_all(1.0, -1.0);

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
    let mut engine = ScriptEngine::new();

    let registry = Arc::new(ComponentRegistry::new());
    let world = Rc::new(RefCell::new(World::new(registry.clone())));
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
