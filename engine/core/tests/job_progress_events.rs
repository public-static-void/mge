#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, assign_jobs};
use serde_json::json;

#[test]
fn test_job_progressed_event_emitted_on_progress_change() {
    let mut world = world_helper::make_test_world();

    // Agent and job setup
    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle"
            }),
        )
        .unwrap();

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "state": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

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
        assert_eq!(
            event.get("entity").and_then(|v| v.as_u64()),
            Some(job_id as u64),
            "Event should reference job entity"
        );
        assert!(
            event.get("progress").is_some(),
            "Event should have progress"
        );
        assert!(event.get("state").is_some(), "Event should have state");
    }

    // There should be no duplicate events for the same progress value
    let mut seen = Vec::new();
    for event in &all_events {
        let progress = event.get("progress").and_then(|v| v.as_f64()).unwrap();
        assert!(
            !seen.contains(&progress),
            "Duplicate progress event for value {progress}"
        );
        seen.push(progress);
    }
}

#[test]
fn test_job_progressed_event_emitted_for_custom_handler() {
    let mut world = world_helper::make_test_world();

    // Register a custom handler that sets progress in two steps
    world.register_job_handler(
        "twostep",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if progress < 1.0 {
                job["progress"] = serde_json::json!(1.0);
                job["state"] = serde_json::json!("in_progress");
            } else {
                job["progress"] = serde_json::json!(2.0);
                job["state"] = serde_json::json!("complete");
            }
            job
        },
    );

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle"
            }),
        )
        .unwrap();

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "twostep",
                "state": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();

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
    assert!(
        progresses.contains(&1.0),
        "Should have progress event for 1.0"
    );
    assert!(
        progresses.contains(&2.0),
        "Should have progress event for 2.0"
    );
}
