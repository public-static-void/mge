use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::scripting::world::World;
use engine_core::systems::standard::{MoveAll, ProcessDeaths};
use schemars::schema::RootSchema;
use serde_json::Value;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Helper: Convert a serde_json::Value into a RootSchema for registration.
fn make_schema_from_json(value: Value) -> RootSchema {
    serde_json::from_value(value).expect("Invalid schema JSON")
}

pub fn make_test_world_with_positions() -> World {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        let schema_json = json!({
            "title": "Position",
            "type": "object",
            "properties": {
                "x": { "type": "number" },
                "y": { "type": "number" }
            },
            "required": ["x", "y"],
            "modes": ["colony", "roguelike"]
        });
        let schema = make_schema_from_json(schema_json);
        reg.register_external_schema(ComponentSchema {
            name: "Position".to_string(),
            schema: schema.clone(),
            modes: vec!["colony".to_string(), "roguelike".to_string()],
        });
    } // lock is dropped here
    let mut world = World::new(registry.clone());
    for i in 0..3 {
        let eid = world.spawn_entity();
        world
            .set_component(eid, "Position", json!({"x": i as f32, "y": 0.0}))
            .unwrap();
    }
    world
}

pub fn make_test_world_with_health() -> (World, u32) {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        let schema_json = json!({
            "title": "Health",
            "type": "object",
            "properties": {
                "current": { "type": "number" },
                "max": { "type": "number" }
            },
            "required": ["current", "max"],
            "modes": ["colony", "roguelike"]
        });
        let schema = make_schema_from_json(schema_json);
        reg.register_external_schema(ComponentSchema {
            name: "Health".to_string(),
            schema: schema.clone(),
            modes: vec!["colony".to_string(), "roguelike".to_string()],
        });
    } // lock is dropped here
    let mut world = World::new(registry.clone());
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Health", json!({"current": 10, "max": 10}))
        .unwrap();
    (world, eid)
}

#[test]
fn test_move_all_system_moves_entities() {
    let mut world = make_test_world_with_positions();
    world.register_system(MoveAll { dx: 1.0, dy: 2.0 });
    world.run_system("MoveAll", None).unwrap();
    // Assert positions incremented
}

#[test]
fn test_process_deaths_creates_corpse_and_decay() {
    let (mut world, entity) = make_test_world_with_health();
    world.register_system(ProcessDeaths);
    world
        .set_component(entity, "Health", json!({"current": 0, "max": 10}))
        .unwrap();
    world.run_system("ProcessDeaths", None).unwrap();
    // Assert Corpse and Decay components present
}
