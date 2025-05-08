use engine_core::scripting::{ScriptEngine, World};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_damage_all_reduces_health() {
    let mut world = World::new();

    let id1 = world.spawn();
    let id2 = world.spawn();

    world
        .set_component(
            id1,
            "Health",
            serde_json::json!({ "current": 10.0, "max": 10.0 }),
        )
        .unwrap();
    world
        .set_component(
            id2,
            "Health",
            serde_json::json!({ "current": 5.0, "max": 8.0 }),
        )
        .unwrap();

    world.damage_all(3.0);

    let health1 = world.get_component(id1, "Health").unwrap();
    let health2 = world.get_component(id2, "Health").unwrap();

    assert!((health1["current"].as_f64().unwrap() - 7.0).abs() < 1e-6);
    assert!((health2["current"].as_f64().unwrap() - 2.0).abs() < 1e-6);
}

#[test]
fn test_lua_damage_all() {
    let mut engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new()));
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local id = spawn_entity()
        set_component(id, "Health", { current = 10.0, max = 10.0 })
        damage_all(4.0)
        local health = get_component(id, "Health")
        assert(math.abs(health.current - 6.0) < 1e-6)
    "#;

    engine.run_script(script).unwrap();
}
