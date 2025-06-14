use std::path::Path;
use std::sync::{Arc, Mutex};

use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::{JobSystem, assign_jobs};
use engine_core::systems::job_board::JobBoard;
use serde_json::json;

fn setup_registry() -> Arc<Mutex<ComponentRegistry>> {
    let config =
        GameConfig::load_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"))
            .expect("Failed to load config");
    let schema_dir = "../../engine/assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    Arc::new(Mutex::new(registry))
}

#[test]
fn test_job_progressed_event_emitted_on_progress_change() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Agent and job setup
    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(1);

    world
        .set_component(
            100,
            "Job",
            json!({
                "id": 100,
                "job_type": "dig",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(100);

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let mut job_system = JobSystem::new();

    // Run job system for several ticks, capturing progress events
    let mut all_events = Vec::new();
    for _ in 0..5 {
        job_system.run(&mut world, None);
        world.update_event_buses::<serde_json::Value>();
        let bus = world
            .get_event_bus::<serde_json::Value>("job_progressed")
            .unwrap();
        let mut reader = engine_core::ecs::event::EventReader::default();
        let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
        all_events.extend(events);
    }

    // There should be at least one progress event, and all should have correct entity and progress
    assert!(!all_events.is_empty(), "No job_progressed events emitted");
    for event in &all_events {
        assert_eq!(event.get("entity").and_then(|v| v.as_u64()), Some(100));
        assert!(event.get("progress").is_some());
        assert!(event.get("status").is_some());
    }

    // There should be no duplicate events for the same progress value
    let mut seen = Vec::new();
    for event in &all_events {
        let progress = event.get("progress").and_then(|v| v.as_f64()).unwrap();
        assert!(
            !seen.contains(&progress),
            "Duplicate progress event for value {}",
            progress
        );
        seen.push(progress);
    }
}

#[test]
fn test_job_progressed_event_emitted_for_custom_handler() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Register a custom handler that sets progress in two steps
    world.register_job_handler(
        "twostep",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if progress < 1.0 {
                job["progress"] = serde_json::json!(1.0);
                job["status"] = serde_json::json!("in_progress");
            } else {
                job["progress"] = serde_json::json!(2.0);
                job["status"] = serde_json::json!("complete");
            }
            job
        },
    );

    world
        .set_component(
            2,
            "Agent",
            json!({
                "entity_id": 2,
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(2);

    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "twostep",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();
    world.entities.push(200);

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let mut job_system = JobSystem::new();

    let mut all_events = Vec::new();
    for _ in 0..3 {
        job_system.run(&mut world, None);
        world.update_event_buses::<serde_json::Value>();
        let bus = world
            .get_event_bus::<serde_json::Value>("job_progressed")
            .unwrap();
        let mut reader = engine_core::ecs::event::EventReader::default();
        let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
        all_events.extend(events);
    }

    // Should have progress events for both 1.0 and 2.0
    let progresses: Vec<_> = all_events
        .iter()
        .map(|e| e.get("progress").and_then(|v| v.as_f64()).unwrap())
        .collect();
    assert!(progresses.contains(&1.0));
    assert!(progresses.contains(&2.0));
}
