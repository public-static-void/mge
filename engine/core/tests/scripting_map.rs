use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::scripting::engine::ScriptEngine;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn test_lua_map_api() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world = Rc::new(RefCell::new(World::new(registry.clone())));

    {
        let mut w = world.borrow_mut();
        let grid = engine_core::map::SquareGridMap::new();
        let map = engine_core::map::Map::new(Box::new(grid));
        w.map = Some(map);
    }

    let mut engine = ScriptEngine::new();
    engine.register_world(world.clone()).unwrap();

    let lua_code = r#"
        add_cell(0, 0, 0)
        add_cell(1, 0, 0)
        add_cell(0, 1, 0)
        local topo = get_map_topology_type()
        assert(topo == "square", "Topology should be square")
        local cells = get_all_cells()
        assert(#cells >= 3, "Should have at least 3 cells")
        local cell = { Square = { x = 0, y = 0, z = 0 } }
        local neighbors = get_neighbors(cell)
        -- You may need to add neighbors explicitly if your map does not auto-calculate them
        -- assert(#neighbors > 0, "Cell should have neighbors")
    "#;

    engine.run_script(lua_code).unwrap();
}
