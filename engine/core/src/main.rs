mod scripting;
use scripting::{ScriptEngine, World};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new()));
    engine.register_world(world.clone()).unwrap();

    // Example Lua script: spawn and move an entity
    let script = r#"
        local id = spawn_entity()
        set_component(id, "Position", { x = 10.0, y = 20.0 })
        local pos = get_component(id, "Position")
        print("Entity " .. id .. " position: x=" .. pos.x .. " y=" .. pos.y)
    "#;

    engine.run_script(script).unwrap();
}
