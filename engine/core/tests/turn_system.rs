use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::scripting::ScriptEngine;
use engine_core::systems::standard::{DamageAll, MoveAll, MoveDelta, ProcessDeaths, ProcessDecay};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn setup_registry() -> Arc<Mutex<ComponentRegistry>> {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    Arc::new(Mutex::new(registry))
}

#[test]
fn test_tick_advances_turn_and_runs_systems() {
    let registry = setup_registry();
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    // Add map with both the initial and target cells
    use engine_core::map::{Map, SquareGridMap};
    let mut grid = SquareGridMap::new();
    grid.add_cell(1, 2, 0); // initial
    grid.add_cell(2, 2, 0); // after move
    world.map = Some(Map::new(Box::new(grid)));

    let id = world.spawn_entity();
    world
        .set_component(
            id,
            "PositionComponent",
            serde_json::json!({ "pos": { "Square": { "x": 1, "y": 2, "z": 0 } } }),
        )
        .unwrap();
    world
        .set_component(
            id,
            "Health",
            serde_json::json!({ "current": 10.0, "max": 10.0 }),
        )
        .unwrap();

    // Set up a tick: move all + damage all
    world.register_system(MoveAll {
        delta: MoveDelta::Square {
            dx: 1,
            dy: 0,
            dz: 0,
        },
    });
    world.run_system("MoveAll", None).unwrap();
    world.register_system(DamageAll { amount: 1.0 });
    world.run_system("DamageAll", None).unwrap();
    world.register_system(ProcessDeaths);
    world.run_system("ProcessDeaths", None).unwrap();
    world.register_system(ProcessDecay);
    world.run_system("ProcessDecay", None).unwrap();
    world.turn += 1;

    // Position should be x+1, Health should be -1
    let pos = world.get_component(id, "PositionComponent").unwrap();
    let health = world.get_component(id, "Health").unwrap();

    assert!((pos["pos"]["Square"]["x"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    assert!((health["current"].as_f64().unwrap() - 9.0).abs() < 1e-6);
    assert_eq!(world.turn, 1);
}

#[test]
fn test_lua_tick() {
    let mut engine = ScriptEngine::new();
    let registry = setup_registry();
    let world = Rc::new(RefCell::new(World::new(registry.clone())));
    world.borrow_mut().current_mode = "colony".to_string();

    // Add map with both the initial and target cells
    use engine_core::map::{Map, SquareGridMap};
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0); // initial
    grid.add_cell(1, 0, 0); // after move
    world.borrow_mut().map = Some(Map::new(Box::new(grid)));

    world.borrow_mut().register_system(MoveAll {
        delta: MoveDelta::Square {
            dx: 1,
            dy: 0,
            dz: 0,
        },
    });
    world
        .borrow_mut()
        .register_system(DamageAll { amount: 1.0 });
    world.borrow_mut().register_system(ProcessDeaths);
    world.borrow_mut().register_system(ProcessDecay);
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local id = spawn_entity()
        set_component(id, "PositionComponent", { pos = { Square = { x = 0, y = 0, z = 0 } } })
        set_component(id, "Health", { current = 10.0, max = 10.0 })
        tick()
        local pos = get_component(id, "PositionComponent")
        local health = get_component(id, "Health")
        assert(math.abs(pos.pos.Square.x - 1.0) < 1e-6)
        assert(math.abs(health.current - 9.0) < 1e-6)
        assert(get_turn() == 1)
    "#;

    engine.run_script(script).unwrap();
}
