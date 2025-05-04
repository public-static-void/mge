use engine_core::scripting::{ScriptEngine, World};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn lua_can_spawn_and_move_entity() {
    let engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new()));
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        function approx(a, b)
            return math.abs(a - b) < 1e-5
        end

        local id = spawn_entity()
        set_position(id, 5.5, 9.9)
        local pos = get_position(id)
        print("pos.x=" .. tostring(pos.x) .. " pos.y=" .. tostring(pos.y))
        assert(approx(pos.x, 5.5))
        assert(approx(pos.y, 9.9))
    "#;

    // Should not panic or error
    engine.run_script(script).unwrap();

    // Also check from Rust side
    let world_ref = world.borrow();
    let entity_id = *world_ref.entities.last().unwrap();
    let pos = world_ref.get_position(entity_id).unwrap();
    assert_eq!(pos.x, 5.5);
    assert_eq!(pos.y, 9.9);
}

#[test]
fn lua_can_run_script_from_file() {
    let engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new()));
    engine.register_world(world.clone()).unwrap();

    let script_path = format!("{}/../scripts/lua/demo.lua", env!("CARGO_MANIFEST_DIR"));
    let script = std::fs::read_to_string(script_path).unwrap();
    engine.run_script(&script).unwrap();

    let world_ref = world.borrow();
    let entity_id = *world_ref.entities.last().unwrap();
    let pos = world_ref.get_position(entity_id).unwrap();
    assert!((pos.x - 1.1).abs() < 1e-5);
    assert!((pos.y - 2.2).abs() < 1e-5);
}

#[test]
fn lua_can_set_and_get_health() {
    let engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new()));
    engine.register_world(world.clone()).unwrap();

    let script_path = format!(
        "{}/../scripts/lua/health_test.lua",
        env!("CARGO_MANIFEST_DIR")
    );
    let script = std::fs::read_to_string(script_path).unwrap();
    engine.run_script(&script).unwrap();

    let world_ref = world.borrow();
    let entity_id = *world_ref.entities.last().unwrap();
    let health = world_ref.get_health(entity_id).unwrap();
    assert!((health.current - 7.0).abs() < 1e-5);
    assert!((health.max - 10.0).abs() < 1e-5);
}
