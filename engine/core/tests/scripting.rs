use engine_core::scripting::{ScriptEngine, World};
use mlua::Lua;
use serde_json::json;
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

#[test]
fn test_get_entities_with_component() {
    let mut world = World::new();
    let id1 = world.spawn();
    let id2 = world.spawn();
    world
        .set_component(id1, "Type", json!({ "kind": "player" }))
        .unwrap();
    world
        .set_component(id2, "Type", json!({ "kind": "enemy" }))
        .unwrap();

    let ids = world.get_entities_with_component("Type");
    assert!(ids.contains(&id1));
    assert!(ids.contains(&id2));
}

#[test]
fn test_move_entity() {
    let mut world = World::new();
    let id = world.spawn();
    world
        .set_component(id, "Position", json!({ "x": 0.0, "y": 0.0 }))
        .unwrap();
    world.move_entity(id, 1.0, 2.0);
    let pos = world.get_component(id, "Position").unwrap();
    assert_eq!(pos["x"], 1.0);
    assert_eq!(pos["y"], 2.0);
}

#[test]
fn test_is_entity_alive() {
    let mut world = World::new();
    let id = world.spawn();
    world
        .set_component(id, "Health", json!({ "current": 5.0, "max": 5.0 }))
        .unwrap();
    assert!(world.is_entity_alive(id));
    world
        .set_component(id, "Health", json!({ "current": 0.0, "max": 5.0 }))
        .unwrap();
    assert!(!world.is_entity_alive(id));
}

#[test]
fn test_damage_entity() {
    let mut world = World::new();
    let id = world.spawn();
    world
        .set_component(id, "Health", json!({ "current": 10.0, "max": 10.0 }))
        .unwrap();

    world.damage_entity(id, 3.0);
    let health = world.get_component(id, "Health").unwrap();
    assert_eq!(health["current"], 7.0);

    // Should not go below zero
    world.damage_entity(id, 10.0);
    let health = world.get_component(id, "Health").unwrap();
    assert_eq!(health["current"], 0.0);
}

#[test]
fn test_count_entities_with_type() {
    let mut world = World::new();
    let player = world.spawn();
    let enemy1 = world.spawn();
    let enemy2 = world.spawn();

    world
        .set_component(player, "Type", json!({ "kind": "player" }))
        .unwrap();
    world
        .set_component(enemy1, "Type", json!({ "kind": "enemy" }))
        .unwrap();
    world
        .set_component(enemy2, "Type", json!({ "kind": "enemy" }))
        .unwrap();

    assert_eq!(world.count_entities_with_type("player"), 1);
    assert_eq!(world.count_entities_with_type("enemy"), 2);

    // Remove one enemy and test again
    world.remove_entity(enemy1);
    assert_eq!(world.count_entities_with_type("enemy"), 1);
}

#[test]
fn test_lua_damage_and_count_entities() {
    let world = Rc::new(RefCell::new(World::new()));
    let engine = ScriptEngine::new();
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local p = spawn_entity()
        set_component(p, "Type", { kind = "player" })
        set_component(p, "Health", { current = 10, max = 10 })

        local e1 = spawn_entity()
        set_component(e1, "Type", { kind = "enemy" })
        set_component(e1, "Health", { current = 5, max = 5 })

        local e2 = spawn_entity()
        set_component(e2, "Type", { kind = "enemy" })
        set_component(e2, "Health", { current = 5, max = 5 })

        damage_entity(e1, 2)
        assert(get_component(e1, "Health").current == 3)

        assert(count_entities_with_type("enemy") == 2)
    "#;
    assert!(engine.run_script(script).is_ok());
}
