#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, assign_jobs};
use serde_json::json;

#[test]
fn test_job_lifecycle_events_emitted() {
    let mut world = world_helper::make_test_world();

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

    // Advance event buses to make assigned event visible
    world.update_event_buses::<serde_json::Value>();

    // Collect assigned event after assignment and update
    let assigned_events = world.drain_events::<serde_json::Value>("job_assigned");

    // Accumulate all events over all ticks
    let mut completed_events: Vec<serde_json::Value> = Vec::new();
    let mut failed_events: Vec<serde_json::Value> = Vec::new();
    let mut cancelled_events: Vec<serde_json::Value> = Vec::new();
    let mut progress_events: Vec<serde_json::Value> = Vec::new();

    let mut job_system = JobSystem::new();
    for _ in 0..5 {
        job_system.run(&mut world, None);

        // Advance event buses after system run, before draining
        world.update_event_buses::<serde_json::Value>();

        completed_events.extend(world.drain_events::<serde_json::Value>("job_completed"));
        failed_events.extend(world.drain_events::<serde_json::Value>("job_failed"));
        cancelled_events.extend(world.drain_events::<serde_json::Value>("job_cancelled"));
        progress_events.extend(world.drain_events::<serde_json::Value>("job_progressed"));
    }

    // Check that job_assigned event was emitted
    assert!(
        assigned_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(100))),
        "No job_assigned event for job 100"
    );
    // Check that job_completed event was emitted
    assert!(
        completed_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(100))),
        "No job_completed event for job 100"
    );
    // Check that at least one progress event was emitted
    assert!(
        progress_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(100))),
        "No job_progressed event for job 100"
    );

    // Check event payloads are rich and consistent
    for event in assigned_events
        .iter()
        .chain(completed_events.iter())
        .chain(progress_events.iter())
    {
        assert!(event.get("entity").is_some());
        assert!(event.get("job_type").is_some());
        assert!(event.get("status").is_some());
    }
}

#[test]
fn test_job_cancel_and_failure_events() {
    let mut world = world_helper::make_test_world();

    // Setup a job that will fail
    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "failtest",
                "status": "pending",
                "should_fail": true,
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();
    world.entities.push(200);

    // Setup a job that will be cancelled
    world
        .set_component(
            201,
            "Job",
            json!({
                "id": 201,
                "job_type": "dig",
                "status": "pending",
                "cancelled": true,
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(201);

    let mut failed_events: Vec<serde_json::Value> = Vec::new();
    let mut cancelled_events: Vec<serde_json::Value> = Vec::new();

    let mut job_system = JobSystem::new();
    for _ in 0..5 {
        job_system.run(&mut world, None);

        // Advance event buses after system run, before draining
        world.update_event_buses::<serde_json::Value>();

        failed_events.extend(world.drain_events::<serde_json::Value>("job_failed"));
        cancelled_events.extend(world.drain_events::<serde_json::Value>("job_cancelled"));
    }

    assert!(
        failed_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(200))),
        "No job_failed event for job 200"
    );
    assert!(
        cancelled_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(201))),
        "No job_cancelled event for job 201"
    );
}
