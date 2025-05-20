use engine_core::ecs::system::{System, SystemRegistry};
use engine_core::ecs::world::World;
use serde_json::json;
use std::sync::{Arc, Mutex};

struct DeathDetector;

impl System for DeathDetector {
    fn name(&self) -> &'static str {
        "DeathDetector"
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        // Phase 1: Collect dead entities
        let dead_entities: Vec<u32> = world
            .components
            .get("Health")
            .map(|healths| {
                healths
                    .iter()
                    .filter_map(|(&entity, value)| {
                        value
                            .as_object()
                            .and_then(|obj| obj.get("current"))
                            .and_then(|current| {
                                if current.as_f64().unwrap_or(1.0) <= 0.0 {
                                    Some(entity)
                                } else {
                                    None
                                }
                            })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Phase 2: Emit events
        for entity in dead_entities {
            world.emit_event("EntityDied", json!({ "entity": entity }));
        }
    }
}

struct DeathProcessor;

impl System for DeathProcessor {
    fn name(&self) -> &'static str {
        "DeathProcessor"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &["DeathDetector"]
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        // Phase 1: Collect entity IDs from events
        let mut to_process = Vec::new();
        world.process_events("EntityDied", |payload| {
            if let Some(entity_val) = payload.get("entity") {
                if let Some(entity) = entity_val.as_u64() {
                    to_process.push(entity as u32);
                }
            }
        });

        // Phase 2: Mutate world
        for entity in to_process {
            if let Some(healths) = world.components.get_mut("Health") {
                healths.remove(&entity);
            }
            let _ = world.set_component(entity, "Corpse", json!({}));
            let _ = world.set_component(entity, "Decay", json!({ "time_remaining": 5 }));
        }
    }
}

#[test]
fn test_death_event_flow() {
    use engine_core::ecs::schema::load_schemas_from_dir;

    // Setup registry and load schemas using an absolute path
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let schema_dir =
        std::env::var("CARGO_MANIFEST_DIR").unwrap().to_string() + "/../assets/schemas";
    println!("Loading schemas from: {}", schema_dir);
    let schemas = load_schemas_from_dir(&schema_dir)
        .unwrap_or_else(|_| panic!("Failed to load schemas from {schema_dir}"));
    {
        let mut reg = registry.lock().unwrap();
        for (_name, schema) in schemas {
            reg.register_external_schema(schema);
        }
    }

    let mut world = World::new(registry.clone());

    // Spawn entity with Health <= 0
    let entity = world.spawn_entity();
    let _ = world.set_component(entity, "Health", json!({ "current": 0, "max": 10 }));

    // Register systems
    let mut sys_registry = SystemRegistry::new();
    sys_registry.register_system(DeathDetector);
    sys_registry.register_system(DeathProcessor);

    // Run tick: DeathDetector emits event, DeathProcessor processes it
    let sorted = sys_registry.sorted_system_names();
    for sys_name in sorted {
        let mut sys = sys_registry.get_system_mut(&sys_name).unwrap();
        sys.run(&mut world, None);
        world.update_event_queues();
    }

    // Assert Health component removed
    assert!(world.get_component(entity, "Health").is_none());

    // Assert Corpse and Decay components added
    assert!(world.get_component(entity, "Corpse").is_some());
    assert!(world.get_component(entity, "Decay").is_some());
}
