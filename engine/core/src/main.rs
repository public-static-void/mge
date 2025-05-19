use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::scripting::ScriptEngine;
use engine_core::systems::inventory::InventoryConstraintSystem;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn main() {
    // Load all schemas!
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    for (_name, schema) in schemas {
        registry.lock().unwrap().register_external_schema(schema);
    }

    let mut engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new(registry.clone())));
    world.borrow_mut().current_mode = "colony".to_string(); // or "roguelike" as needed
    engine.register_world(world.clone()).unwrap();

    world
        .borrow_mut()
        .register_system(InventoryConstraintSystem);

    // Example Lua script: spawn and move an entity
    let script = r#"
        local id = spawn_entity()
        set_component(id, "Position", { x = 10.0, y = 20.0 })
        local pos = get_component(id, "Position")
        print("Entity " .. id .. " position: x=" .. pos.x .. " y=" .. pos.y)
    "#;

    engine.run_script(script).unwrap();
}
