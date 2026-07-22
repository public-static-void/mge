#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::event::{EventBus, EventReader};
use engine_core::ecs::event_logger::EventLogger;
use engine_core::ecs::system::{System, SystemRegistry};
use engine_core::ecs::world::World;
use engine_core::systems::death_decay::{ProcessDeaths, ProcessDecay};
use serde_json::json;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// === event_system tests ===

#[derive(Clone, Debug, PartialEq)]
struct TestEvent {
    pub value: i32,
}

#[test]
fn test_event_send_and_receive() {
    let mut bus = EventBus::<TestEvent>::default();
    let mut reader = EventReader::new();

    assert_eq!(reader.read(&bus).count(), 0);

    bus.send(TestEvent { value: 42 });
    bus.update();

    let events: Vec<_> = reader.read(&bus).cloned().collect();
    assert_eq!(events, vec![TestEvent { value: 42 }]);

    assert_eq!(reader.read(&bus).count(), 0);
}

// === event_logger tests ===

#[test]
fn test_event_logger_query_by_type() {
    let logger = EventLogger::<serde_json::Value>::new();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));
    logger.log("job_completed", json!({"id": 3}));

    let events = logger.query_events(|e| e.event_type == "job_completed");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].payload["id"], 1);
    assert_eq!(events[1].payload["id"], 3);
}

#[test]
fn test_event_logger_query_by_time_range() {
    let logger = EventLogger::<serde_json::Value>::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));

    let events = logger.query_events(|e| e.timestamp >= now);
    assert_eq!(events.len(), 2);
}

#[test]
fn test_event_logger_query_by_payload_field() {
    let logger = EventLogger::<serde_json::Value>::new();
    logger.log("job_completed", json!({"id": 1, "priority": 10}));
    logger.log("job_failed", json!({"id": 2, "priority": 1}));
    logger.log("job_completed", json!({"id": 3, "priority": 10}));

    let events =
        logger.query_events(|e| e.payload.get("priority").and_then(|v| v.as_i64()) == Some(10));
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].payload["id"], 1);
    assert_eq!(events[1].payload["id"], 3);
}

#[test]
fn test_get_events_by_type() {
    let logger = EventLogger::<serde_json::Value>::new();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));
    logger.log("job_completed", json!({"id": 3}));

    let events = logger.get_events_by_type("job_completed");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].payload["id"], 1);
    assert_eq!(events[1].payload["id"], 3);
}

#[test]
fn test_get_events_since() {
    let logger = EventLogger::<serde_json::Value>::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));

    let events = logger.get_events_since(now);
    assert_eq!(events.len(), 2);
}

#[test]
fn test_get_events_where() {
    let logger = EventLogger::<serde_json::Value>::new();
    logger.log("job_completed", json!({"id": 1, "priority": 10}));
    logger.log("job_failed", json!({"id": 2, "priority": 1}));
    logger.log("job_completed", json!({"id": 3, "priority": 10}));

    let events =
        logger.get_events_where(|p| p.get("priority").and_then(|v| v.as_i64()) == Some(10));
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].payload["id"], 1);
    assert_eq!(events[1].payload["id"], 3);
}

#[test]
fn test_event_logger_log_and_replay() {
    let logger = EventLogger::new();

    logger.log("job_assigned", json!({"entity": 1, "state": "in_progress"}));
    logger.log("job_completed", json!({"entity": 1, "state": "complete"}));

    let events = logger.all();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "job_assigned");
    assert_eq!(events[1].event_type, "job_completed");

    let mut replayed = Vec::new();
    logger.replay_into(|e| replayed.push(e.event_type.clone()));
    assert_eq!(replayed, vec!["job_assigned", "job_completed"]);
}

#[test]
fn test_event_logger_save_and_load() {
    let logger = EventLogger::new();
    logger.log("job_failed", json!({"entity": 2, "state": "failed"}));

    let path = "test_event_log.json";
    logger.save_to_file(path).unwrap();

    let loaded: EventLogger<serde_json::Value> = EventLogger::load_from_file(path).unwrap();
    fs::remove_file(path).unwrap();

    let events = loaded.all();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "job_failed");
    assert_eq!(events[0].payload["state"], "failed");
}

// === event_logger_integration tests ===

#[test]
fn test_job_event_logging_integration() {
    use engine_core::systems::job::system::events::{
        emit_job_event, init_job_event_logger, job_event_logger,
    };

    init_job_event_logger();
    let mut world = world_helper::make_test_world();

    emit_job_event(&mut world, "job_completed", &json!({"id": 1}), None);

    let events = job_event_logger().get_events_by_type("job_completed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload["entity"], 1);
}

// === process_deaths_event tests ===

struct DeathDetector;

impl System for DeathDetector {
    fn name(&self) -> &'static str {
        "DeathDetector"
    }
    fn run(&mut self, world: &mut World) {
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
    fn run(&mut self, world: &mut World) {
        let mut to_process = Vec::new();
        world.process_events("EntityDied", |payload| {
            if let Some(entity_val) = payload.get("entity")
                && let Some(entity) = entity_val.as_u64()
            {
                to_process.push(entity as u32);
            }
        });

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
    use engine_core::config::GameConfig;
    use engine_core::ecs::schema::load_schemas_from_dir_with_modes;

    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schema_dir =
        std::env::var("CARGO_MANIFEST_DIR").unwrap().to_string() + "/../assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &config.allowed_modes)
        .unwrap_or_else(|_| panic!("Failed to load schemas from {schema_dir}"));
    {
        let mut reg = registry.lock().unwrap();
        for (_name, schema) in schemas {
            reg.register_external_schema(schema);
        }
    }

    let mut world = World::new(registry.clone());

    let entity = world.spawn_entity();
    let _ = world.set_component(entity, "Health", json!({ "current": 0, "max": 10 }));

    let mut sys_registry = SystemRegistry::new();
    sys_registry.register_system(DeathDetector);
    sys_registry.register_system(DeathProcessor);

    let sorted = sys_registry.sorted_system_names();
    for sys_name in sorted {
        let mut sys = sys_registry.get_system_mut(&sys_name).unwrap();
        sys.run(&mut world);
        world.update_event_queues();
    }

    assert!(world.get_component(entity, "Health").is_none());
    assert!(world.get_component(entity, "Corpse").is_some());
    assert!(world.get_component(entity, "Decay").is_some());
}

// === death_removal tests ===

#[test]
fn test_death_replaces_health_with_corpse_and_decay() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let id = world.spawn_entity();
    world
        .set_component(id, "Health", json!({ "current": 1.0, "max": 10.0 }))
        .unwrap();

    if let Some(healths) = world.components.get_mut("Health") {
        for value in healths.values_mut() {
            if let Some(obj) = value.as_object_mut()
                && let Some(current) = obj.get_mut("current")
                && let Some(cur_val) = current.as_f64()
            {
                let new_val = (cur_val - 2.0).max(0.0);
                *current = serde_json::json!(new_val);
            }
        }
    }

    world.register_system(ProcessDeaths);
    world.run_system("ProcessDeaths").unwrap();

    assert!(world.get_component(id, "Health").is_none());
    assert!(world.get_component(id, "Corpse").is_some());

    let decay = world.get_component(id, "Decay").unwrap();
    assert_eq!(decay["time_remaining"].as_u64().unwrap(), 5);
}

#[test]
fn test_decay_removes_entity_after_time() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let id = world.spawn_entity();
    world.set_component(id, "Corpse", json!({})).unwrap();
    world
        .set_component(id, "Decay", json!({ "time_remaining": 2 }))
        .unwrap();

    world.register_system(ProcessDecay);
    world.run_system("ProcessDecay").unwrap();
    let decay = world.get_component(id, "Decay").unwrap();
    assert_eq!(decay["time_remaining"].as_u64().unwrap(), 1);

    world.register_system(ProcessDecay);
    world.run_system("ProcessDecay").unwrap();
    assert!(world.get_component(id, "Decay").is_none());
}
