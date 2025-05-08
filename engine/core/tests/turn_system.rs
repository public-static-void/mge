use engine_core::scripting::{ScriptEngine, World};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_tick_advances_turn_and_runs_systems() {
    let mut world = World::new();

    let id = world.spawn();
    world
        .set_component(id, "Position", serde_json::json!({ "x": 0.0, "y": 0.0 }))
        .unwrap();
    world
        .set_component(
            id,
            "Health",
            serde_json::json!({ "current": 10.0, "max": 10.0 }),
        )
        .unwrap();

    // Set up a tick: move all + damage all
    world.tick();

    // Position should be x+1, Health should be -1
    let pos = world.get_component(id, "Position").unwrap();
    let health = world.get_component(id, "Health").unwrap();

    assert!((pos["x"].as_f64().unwrap() - 1.0).abs() < 1e-6);
    assert!((health["current"].as_f64().unwrap() - 9.0).abs() < 1e-6);
    assert_eq!(world.turn, 1);
}

#[test]
fn test_lua_tick() {
    let mut engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new()));
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local id = spawn_entity()
        set_component(id, "Position", { x = 0.0, y = 0.0 })
        set_component(id, "Health", { current = 10.0, max = 10.0 })
        tick()
        local pos = get_component(id, "Position")
        local health = get_component(id, "Health")
        assert(math.abs(pos.x - 1.0) < 1e-6)
        assert(math.abs(health.current - 9.0) < 1e-6)
        assert(get_turn() == 1)
    "#;

    engine.run_script(script).unwrap();
}
