use engine_core::ecs::world::World;
use engine_core::scripting::lua_api::register_all_api_functions;
use engine_core::scripting::lua_api::worldgen::register_worldgen_api;

use engine_core::worldgen::{WorldgenRegistry, register_builtin_worldgen_plugins};
use mlua::{Lua, Table};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

struct DummyInputProvider;
impl engine_core::scripting::input::InputProvider for DummyInputProvider {
    fn read_line(&mut self, _prompt: &str) -> Result<String, std::io::Error> {
        Ok("".to_string())
    }
}

#[test]
fn test_lua_can_list_and_invoke_worldgen_plugins() {
    // Set up worldgen registry and register builtins (single-threaded, so Rc is fine)
    let mut reg = WorldgenRegistry::new();
    register_builtin_worldgen_plugins(&mut reg);
    let worldgen_registry = Rc::new(RefCell::new(reg));

    // Set up ECS world and Lua (must use Arc<Mutex<...>> as required by World::new)
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let world = Rc::new(RefCell::new(World::new(Arc::clone(&registry))));
    let lua = Lua::new();
    let globals = lua.globals();

    // Set up dummy input provider (must use Arc<Mutex<...>> as required by register_api_functions)
    let input_provider = Arc::new(Mutex::new(Box::new(DummyInputProvider) as Box<_>));

    // Register core ECS API
    register_all_api_functions(
        &lua,
        &globals,
        world.clone(),
        Arc::clone(&input_provider),
        Rc::clone(&worldgen_registry),
    )
    .unwrap();

    // Register worldgen API (Rc-based)
    register_worldgen_api(&lua, &globals, Rc::clone(&worldgen_registry)).unwrap();

    // Lua: list worldgen plugins
    let plugins: Vec<String> = lua
        .load(
            r#"
            return list_worldgen_plugins()
        "#,
        )
        .eval()
        .expect("Should list plugins");
    assert!(plugins.contains(&"basic_square_worldgen".to_string()));

    // Lua: invoke worldgen plugin
    let map: Table = lua
        .load(
            r#"
            local params = { topology = "square", width = 2, height = 2, z_levels = 1, seed = 42 }
            return invoke_worldgen("basic_square_worldgen", params)
        "#,
        )
        .eval()
        .expect("Should invoke worldgen");

    assert_eq!(map.get::<String>("topology").unwrap(), "square");
    let cells: Table = map.get("cells").unwrap();
    assert_eq!(cells.len().unwrap(), 4);
}
