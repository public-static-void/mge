use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::scripting::ScriptEngine;
use engine_core::scripting::input::InputProvider;
use serde_json::json;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// === Helper Functions ===

fn setup_world_with_mode(mode: &str) -> Rc<RefCell<World>> {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    let world = Rc::new(RefCell::new(World::new(registry.clone())));
    world.borrow_mut().current_mode = mode.to_string();
    world
}

fn setup_registry() -> Arc<Mutex<ComponentRegistry>> {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    Arc::new(Mutex::new(registry))
}

// === Mock Input ===

pub struct MockInput {
    inputs: Mutex<VecDeque<String>>,
}

impl MockInput {
    pub fn new(inputs: Vec<String>) -> Self {
        Self {
            inputs: Mutex::new(inputs.into()),
        }
    }
}

impl InputProvider for MockInput {
    fn read_line(&mut self, _prompt: &str) -> Result<String, std::io::Error> {
        self.inputs.lock().unwrap().pop_front().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "No more mock inputs")
        })
    }
}

// === Tests ===

#[test]
fn lua_can_spawn_and_move_entity() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::scripting::ScriptEngine;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let schema_dir = std::path::Path::new("../assets/schemas");
    for entry in std::fs::read_dir(schema_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let json = std::fs::read_to_string(&path).unwrap();
            registry.register_external_schema_from_json(&json).unwrap();
        }
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut engine = ScriptEngine::new();
    let world = std::rc::Rc::new(std::cell::RefCell::new(engine_core::ecs::World::new(
        registry.clone(),
    )));
    world.borrow_mut().current_mode = "roguelike".to_string();
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local id = spawn_entity()
        set_component(id, "PositionComponent", { pos = { Square = { x = 1, y = 2, z = 0 } } })
        local pos = get_component(id, "PositionComponent")
        assert(pos.pos.Square.x == 1)
        assert(pos.pos.Square.y == 2)
        move_entity(id, 3, 4)
        local pos2 = get_component(id, "PositionComponent")
        assert(pos2.pos.Square.x == 4)
        assert(pos2.pos.Square.y == 6)
    "#;
    engine.run_script(script).unwrap();
}

#[test]
fn lua_can_run_script_from_file() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::scripting::ScriptEngine;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let schema_dir = std::path::Path::new("../assets/schemas");
    for entry in std::fs::read_dir(schema_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let json = std::fs::read_to_string(&path).unwrap();
            registry.register_external_schema_from_json(&json).unwrap();
        }
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut engine = ScriptEngine::new();
    let world = std::rc::Rc::new(std::cell::RefCell::new(engine_core::ecs::World::new(
        registry.clone(),
    )));
    world.borrow_mut().current_mode = "roguelike".to_string();
    engine.register_world(world.clone()).unwrap();

    // Write a temp Lua script file
    let mut file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    writeln!(
        file,
        r#"
        local id = spawn_entity()
        set_component(id, "PositionComponent", {{ pos = {{ Square = {{ x = 9, y = 10, z = 0 }} }} }})
        local pos = get_component(id, "PositionComponent")
        assert(pos.pos.Square.x == 9)
        assert(pos.pos.Square.y == 10)
        "#
    )
    .unwrap();

    let script_str = std::fs::read_to_string(file.path()).unwrap();
    engine.run_script(&script_str).unwrap();
}

#[test]
fn lua_can_set_and_get_health() {
    use std::env;
    use std::path::PathBuf;

    // Set LUA_PATH for Lua interpreter
    let lua_test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("scripts/lua/tests");
    let lua_path = format!("{}/?.lua;;", lua_test_dir.display());
    unsafe {
        env::set_var("LUA_PATH", &lua_path);
    }

    let mut engine = ScriptEngine::new();
    let world = setup_world_with_mode("roguelike");
    engine.register_world(world.clone()).unwrap();

    let script_path = lua_test_dir.join("test_health.lua");
    let script = std::fs::read_to_string(&script_path).unwrap();
    engine.run_script(&script).unwrap();

    let world_ref = world.borrow();
    let entity_id = *world_ref.entities.last().unwrap();
    let health = world_ref.get_component(entity_id, "Health").unwrap();
    assert!((health["current"].as_f64().unwrap() - 7.0).abs() < 1e-5);
    assert!((health["max"].as_f64().unwrap() - 10.0).abs() < 1e-5);
}

#[test]
fn lua_can_set_and_get_arbitrary_component() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::scripting::ScriptEngine;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let schema_dir = std::path::Path::new("../assets/schemas");
    for entry in std::fs::read_dir(schema_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let json = std::fs::read_to_string(&path).unwrap();
            registry.register_external_schema_from_json(&json).unwrap();
        }
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut engine = ScriptEngine::new();
    let world = std::rc::Rc::new(std::cell::RefCell::new(engine_core::ecs::World::new(
        registry.clone(),
    )));
    world.borrow_mut().current_mode = "roguelike".to_string();
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local id = spawn_entity()
        set_component(id, "PositionComponent", { pos = { Square = { x = 7, y = 8, z = 0 } } })
        local pos = get_component(id, "PositionComponent")
        assert(pos.pos.Square.x == 7)
        assert(pos.pos.Square.y == 8)
    "#;
    engine.run_script(script).unwrap();
}

#[test]
fn test_lua_component_access_mode_enforcement() {
    let mut engine = ScriptEngine::new();
    let world = setup_world_with_mode("colony");
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        set_mode("colony")
        local id = spawn_entity()
        assert(set_component(id, "Happiness", { base_value = 0.7 }) == true)
        assert(set_component(id, "Inventory", { slots = {}, weight = 1.5 }) == true)
    "#;
    assert!(engine.run_script(script).is_ok());
}

#[test]
fn test_get_entities_with_component() {
    let registry = setup_registry();
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();
    let id1 = world.spawn_entity();
    let id2 = world.spawn_entity();
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
    use engine_core::ecs::World;
    use engine_core::ecs::registry::ComponentRegistry;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let schema_dir = std::path::Path::new("../assets/schemas");
    for entry in std::fs::read_dir(schema_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let json = std::fs::read_to_string(&path).unwrap();
            registry.register_external_schema_from_json(&json).unwrap();
        }
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    let id = world.spawn_entity();
    world
        .set_component(
            id,
            "PositionComponent",
            serde_json::json!({ "pos": { "Square": { "x": 1, "y": 2, "z": 0 } } }),
        )
        .unwrap();

    world.move_entity(id, 2.0, 3.0);

    let pos = world.get_component(id, "PositionComponent").unwrap();
    assert_eq!(pos["pos"]["Square"]["x"].as_i64().unwrap(), 3);
    assert_eq!(pos["pos"]["Square"]["y"].as_i64().unwrap(), 5);
}

#[test]
fn test_is_entity_alive() {
    let registry = setup_registry();
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();
    let id = world.spawn_entity();
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
    let registry = setup_registry();
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();
    let id = world.spawn_entity();
    world
        .set_component(id, "Health", json!({ "current": 10.0, "max": 10.0 }))
        .unwrap();

    world.damage_entity(id, 3.0);
    let health = world.get_component(id, "Health").unwrap();
    assert_eq!(health["current"], 7.0);

    world.damage_entity(id, 10.0);
    let health = world.get_component(id, "Health").unwrap();
    assert_eq!(health["current"], 0.0);
}

#[test]
fn test_count_entities_with_type() {
    let registry = setup_registry();
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();
    let player = world.spawn_entity();
    let enemy1 = world.spawn_entity();
    let enemy2 = world.spawn_entity();

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

    world.despawn_entity(enemy1);
    assert_eq!(world.count_entities_with_type("enemy"), 1);
}

#[test]
fn test_lua_damage_and_count_entities() {
    let mut engine = ScriptEngine::new();
    let world = setup_world_with_mode("roguelike");
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

#[test]
fn test_lua_get_user_input_with_mock() {
    let inputs = vec!["hello".to_string()];
    let mock_input = Box::new(MockInput::new(inputs));

    let world = setup_world_with_mode("roguelike");
    let mut engine = ScriptEngine::new_with_input(mock_input);
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local input = get_user_input("Prompt: ")
        assert(input == "hello")
    "#;

    assert!(engine.run_script(script).is_ok());
}
