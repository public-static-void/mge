use engine_lua::{ScriptEngine, input::InputProvider};
use std::cell::RefCell;
use std::rc::Rc;

pub struct MockInput {
    value: String,
}
impl InputProvider for MockInput {
    fn read_line(&mut self, _prompt: &str) -> Result<String, std::io::Error> {
        Ok(self.value.clone())
    }
}

#[test]
fn lua_get_user_input_returns_mocked_value() {
    // The value to be returned by get_user_input
    let expected = "hello".to_string();
    let mock_input = Box::new(MockInput {
        value: expected.clone(),
    });
    let mut engine = ScriptEngine::new_with_input(mock_input);

    // Register a dummy world if required by your engine
    let registry = engine_core::ecs::registry::ComponentRegistry::new();
    let world = Rc::new(RefCell::new(engine_core::ecs::World::new(
        std::sync::Arc::new(std::sync::Mutex::new(registry)),
    )));
    world.borrow_mut().current_mode = "roguelike".to_string();
    engine.register_world(world.clone()).unwrap();

    // (Optional) Set a global in Lua for parameterized checks in the Lua test
    engine
        .lua
        .globals()
        .set("TEST_EXPECTED_INPUT", expected.clone())
        .unwrap();

    // Set Lua's package.path so 'require' works for test modules
    let lua_tests_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../engine/scripts/lua/tests");
    let lua_tests_dir_str = lua_tests_dir.to_str().unwrap();
    let package: mlua::Table = engine.lua.globals().get("package").unwrap();
    let old_path: String = package.get("path").unwrap();
    let new_path = format!("{lua_tests_dir_str}/?.lua;{old_path}");
    package.set("path", new_path).unwrap();

    let script_path = lua_tests_dir.join("test_input.lua");
    let script = std::fs::read_to_string(&script_path).unwrap();
    engine.run_script(&script).unwrap();
}
