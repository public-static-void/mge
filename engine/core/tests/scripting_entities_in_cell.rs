use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::scripting::engine::ScriptEngine;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn test_lua_entities_in_cell_api() {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    for (_name, schema) in schemas {
        registry.lock().unwrap().register_external_schema(schema);
    }

    let world = Rc::new(RefCell::new(World::new(registry.clone())));
    {
        let mut w = world.borrow_mut();
        let mut grid = engine_core::map::SquareGridMap::new();
        grid.add_cell(0, 0, 0);
        grid.add_cell(1, 0, 0);
        grid.add_neighbor((0, 0, 0), (1, 0, 0));
        let map = engine_core::map::Map::new(Box::new(grid));
        w.map = Some(map);
    }

    let mut engine = ScriptEngine::new();
    engine.register_world(world.clone()).unwrap();

    let lua_code = r#"
        local eid = spawn_entity()
        set_component(eid, "Position", { pos = { Square = { x = 0, y = 0, z = 0 } } })
        local cell = { Square = { x = 0, y = 0, z = 0 } }
        local entities = entities_in_cell(cell)
        assert(#entities == 1, "Should find one entity in cell")
    "#;

    engine.run_script(lua_code).unwrap();
}
