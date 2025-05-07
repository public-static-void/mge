use engine_core::scripting::{ScriptEngine, World};
use mlua::Lua;
use std::cell::RefCell;
use std::rc::Rc;

fn setup_engine_with_modes(_lua: &mlua::Lua) -> ScriptEngine {
    let engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new()));
    // Optionally: register components or set up modes here if needed
    engine.register_world(world).unwrap();
    engine
}

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
        set_component(id, "Position", { x = 5.5, y = 9.9 })
        local pos = get_component(id, "Position")
        print("pos.x=" .. tostring(pos.x) .. " pos.y=" .. tostring(pos.y))
        assert(approx(pos.x, 5.5))
        assert(approx(pos.y, 9.9))
    "#;

    // Should not panic or error
    engine.run_script(script).unwrap();

    // Also check from Rust side
    let world_ref = world.borrow();
    let entity_id = *world_ref.entities.last().unwrap();
    let pos = world_ref.get_component(entity_id, "Position").unwrap();
    assert!((pos["x"].as_f64().unwrap() - 5.5).abs() < 1e-5);
    assert!((pos["y"].as_f64().unwrap() - 9.9).abs() < 1e-5);
}

#[test]
fn lua_can_run_script_from_file() {
    let engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new()));
    engine.register_world(world.clone()).unwrap();

    let script_path = format!(
        "{}/../scripts/lua/position_demo.lua",
        env!("CARGO_MANIFEST_DIR")
    );
    let script = std::fs::read_to_string(script_path).unwrap();
    engine.run_script(&script).unwrap();

    let world_ref = world.borrow();
    let entity_id = *world_ref.entities.last().unwrap();
    let pos = world_ref.get_component(entity_id, "Position").unwrap();
    assert!((pos["x"].as_f64().unwrap() - 1.1).abs() < 1e-5);
    assert!((pos["y"].as_f64().unwrap() - 2.2).abs() < 1e-5);
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
    let health = world_ref.get_component(entity_id, "Health").unwrap();
    assert!((health["current"].as_f64().unwrap() - 7.0).abs() < 1e-5);
    assert!((health["max"].as_f64().unwrap() - 10.0).abs() < 1e-5);
}

#[test]
fn lua_can_set_and_get_arbitrary_component() {
    use engine_core::scripting::{ScriptEngine, World};
    use std::cell::RefCell;
    use std::rc::Rc;

    let engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new()));
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local id = spawn_entity()
        set_component(id, "Position", { x = 42.0, y = 99.0 })
        local pos = get_component(id, "Position")
        assert(math.abs(pos.x - 42.0) < 1e-5)
        assert(math.abs(pos.y - 99.0) < 1e-5)

        set_component(id, "Health", { current = 7.5, max = 10.0 })
        local health = get_component(id, "Health")
        assert(math.abs(health.current - 7.5) < 1e-5)
        assert(math.abs(health.max - 10.0) < 1e-5)
    "#;

    // Should not panic or error
    engine.run_script(script).unwrap();

    // Also check from Rust side (optional)
    let world_ref = world.borrow();
    let entity_id = *world_ref.entities.last().unwrap();
    let pos = world_ref.get_component(entity_id, "Position").unwrap();
    assert!((pos["x"].as_f64().unwrap() - 42.0).abs() < 1e-5);
    assert!((pos["y"].as_f64().unwrap() - 99.0).abs() < 1e-5);

    let health = world_ref.get_component(entity_id, "Health").unwrap();
    assert!((health["current"].as_f64().unwrap() - 7.5).abs() < 1e-5);
    assert!((health["max"].as_f64().unwrap() - 10.0).abs() < 1e-5);
}

#[test]
fn test_lua_component_access_mode_enforcement() {
    let lua = Lua::new();
    let engine = setup_engine_with_modes(&lua);

    // Lua script: in "colony" mode, try to set colony and roguelike components
    let script = r#"
        set_mode("colony")
        local id = spawn_entity()
        assert(set_component(id, "Colony::Happiness", { base_value = 0.7 }) == true)
        local ok, err = pcall(function()
            set_component(id, "Roguelike::Inventory", { slots = 4, weight = 1.5 })
        end)
        assert(ok == false)
    "#;
    assert!(engine.run_script(script).is_ok());
}
