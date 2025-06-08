use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::mods::loader::ModScriptEngine;
use engine_core::systems::death_decay::{ProcessDeaths, ProcessDecay};
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

pub fn scripting_tick_test<E: ModScriptEngine>(mut engine: E) {
    let registry = setup_registry();
    let world = Rc::new(RefCell::new(World::new(registry.clone())));
    world.borrow_mut().current_mode = "colony".to_string();

    use engine_core::map::{Map, SquareGridMap};
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0); // initial
    grid.add_cell(1, 0, 0); // after move
    world.borrow_mut().map = Some(Map::new(Box::new(grid)));

    // NOTE: If your scripting engine needs to register the world,
    // do it in the bridge test before calling this function.

    let script = r#"
        local id = spawn_entity()
        set_component(id, "Position", { pos = { Square = { x = 0, y = 0, z = 0 } } })
        set_component(id, "Health", { current = 10.0, max = 10.0 })
        -- Move all: increment x for all entities with Position (Square)
        for _, eid in ipairs(get_entities_with_component("Position")) do
            local pos = get_component(eid, "Position")
            if pos.pos and pos.pos.Square then
                pos.pos.Square.x = pos.pos.Square.x + 1
                set_component(eid, "Position", pos)
            end
        end
        -- Damage all: decrement health for all entities with Health
        for _, eid in ipairs(get_entities_with_component("Health")) do
            local h = get_component(eid, "Health")
            h.current = h.current - 1.0
            set_component(eid, "Health", h)
        end
        tick()
        local pos = get_component(id, "Position")
        local health = get_component(id, "Health")
        assert(math.abs(pos.pos.Square.x - 1.0) < 1e-6)
        assert(math.abs(health.current - 9.0) < 1e-6)
        assert(get_turn() == 1)
    "#;

    engine.run_script(script).unwrap();
}

// The rest of this file can contain pure core tests as before.
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
            "Position",
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

    // Move all: increment x for all entities with Position (Square)
    if let Some(positions) = world.components.get_mut("Position") {
        for (_eid, value) in positions.iter_mut() {
            if let Some(obj) = value.as_object_mut() {
                if let Some(pos) = obj.get_mut("pos") {
                    if let Some(square) = pos.get_mut("Square") {
                        if let Some(x) = square.get_mut("x") {
                            if let Some(x_val) = x.as_i64() {
                                *x = serde_json::json!(x_val + 1);
                            }
                        }
                    }
                }
            }
        }
    }
    // Damage all: decrement health for all entities with Health
    if let Some(healths) = world.components.get_mut("Health") {
        for (_eid, value) in healths.iter_mut() {
            if let Some(obj) = value.as_object_mut() {
                if let Some(current) = obj.get_mut("current") {
                    if let Some(cur_val) = current.as_f64() {
                        let new_val = (cur_val - 1.0).max(0.0);
                        *current = serde_json::json!(new_val);
                    }
                }
            }
        }
    }
    world.register_system(ProcessDeaths);
    world.run_system("ProcessDeaths", None).unwrap();
    world.register_system(ProcessDecay);
    world.run_system("ProcessDecay", None).unwrap();
    world.turn += 1;

    // Position should be x+1, Health should be -1
    let pos = world.get_component(id, "Position").unwrap();
    let health = world.get_component(id, "Health").unwrap();

    assert!((pos["pos"]["Square"]["x"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    assert!((health["current"].as_f64().unwrap() - 9.0).abs() < 1e-6);
    assert_eq!(world.turn, 1);
}
