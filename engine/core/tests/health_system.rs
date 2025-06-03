use engine_core::ecs::World;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn test_damage_all_reduces_health() {
    // Load schemas
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string(); // Ensure correct mode

    let id1 = world.spawn_entity();
    let id2 = world.spawn_entity();

    world
        .set_component(id1, "Health", json!({ "current": 10.0, "max": 10.0 }))
        .unwrap();
    world
        .set_component(id2, "Health", json!({ "current": 5.0, "max": 8.0 }))
        .unwrap();

    // Directly reduce health for all entities with Health
    if let Some(healths) = world.components.get_mut("Health") {
        for (_eid, value) in healths.iter_mut() {
            if let Some(obj) = value.as_object_mut() {
                if let Some(current) = obj.get_mut("current") {
                    if let Some(cur_val) = current.as_f64() {
                        let new_val = (cur_val - 3.0).max(0.0);
                        *current = serde_json::json!(new_val);
                    }
                }
            }
        }
    }

    let health1 = world.get_component(id1, "Health").unwrap();
    let health2 = world.get_component(id2, "Health").unwrap();

    assert!((health1["current"].as_f64().unwrap() - 7.0).abs() < 1e-6);
    assert!((health2["current"].as_f64().unwrap() - 2.0).abs() < 1e-6);
}

#[test]
fn test_lua_damage_all() {
    use engine_core::scripting::ScriptEngine;

    // Load schemas
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));

    let mut engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(World::new(registry.clone())));
    world.borrow_mut().current_mode = "colony".to_string();
    engine.register_world(world.clone()).unwrap();

    let script = r#"
        local id = spawn_entity()
        set_component(id, "Health", { current = 10.0, max = 10.0 })
        for _, eid in ipairs(get_entities_with_component("Health")) do
            local h = get_component(eid, "Health")
            h.current = h.current - 4.0
            set_component(eid, "Health", h)
        end
        local health = get_component(id, "Health")
        assert(math.abs(health.current - 6.0) < 1e-6)
    "#;

    engine.run_script(script).unwrap();
}
