use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::scripting::input::InputProvider;
use engine_core::scripting::{ScriptEngine, World};
use serde_json::json;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

// === Helper Functions ===

fn setup_world_with_mode(mode: &str) -> Rc<RefCell<World>> {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(registry);
    let world = Rc::new(RefCell::new(World::new(registry.clone())));
    world.borrow_mut().current_mode = mode.to_string();
    world
}

fn setup_registry() -> Arc<ComponentRegistry> {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    Arc::new(registry)
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
    let mut engine = ScriptEngine::new();
    let world = setup_world_with_mode("roguelike");
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

    engine.run_script(script).unwrap();

    let world_ref = world.borrow();
    let entity_id = *world_ref.entities.last().unwrap();
    let pos = world_ref.get_component(entity_id, "Position").unwrap();
    assert!((pos["x"].as_f64().unwrap() - 5.5).abs() < 1e-5);
    assert!((pos["y"].as_f64().unwrap() - 9.9).abs() < 1e-5);
}

#[test]
fn lua_can_run_script_from_file() {
    let mut engine = ScriptEngine::new();
    let world = setup_world_with_mode("roguelike");
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
    let mut engine = ScriptEngine::new();
    let world = setup_world_with_mode("roguelike");
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
    let mut engine = ScriptEngine::new();
    let world = setup_world_with_mode("roguelike");
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

    engine.run_script(script).unwrap();

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
    let registry = setup_registry();
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();
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
    let registry = setup_registry();
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();
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
    let registry = setup_registry();
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();
    let id = world.spawn();
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

    world.remove_entity(enemy1);
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
