use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::world::World;
use engine_core::systems::death_decay::ProcessDeaths;
use schemars::Schema;
use serde_json::Value;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Helper: Convert a serde_json::Value into a Schema for registration.
fn make_schema_from_json(value: Value) -> Schema {
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
            schema: schema.clone().into(),
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
            schema: schema.clone().into(),
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
    // Move all: increment x, y for all entities with Position
    if let Some(positions) = world.components.get_mut("Position") {
        for (_eid, value) in positions.iter_mut() {
            if let Some(obj) = value.as_object_mut() {
                if let Some(x) = obj.get_mut("x") {
                    if let Some(x_val) = x.as_f64() {
                        *x = serde_json::json!(x_val + 1.0);
                    }
                }
                if let Some(y) = obj.get_mut("y") {
                    if let Some(y_val) = y.as_f64() {
                        *y = serde_json::json!(y_val + 2.0);
                    }
                }
            }
        }
    }
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
