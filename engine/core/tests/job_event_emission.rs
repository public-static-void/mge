#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, assign_jobs};
use serde_json::json;

#[test]
fn test_job_lifecycle_events_emitted() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register a handler for "dig" jobs so the job can progress and complete
    {
        let registry = world.job_handler_registry.clone();
        registry
            .lock()
            .unwrap()
            .register_handler("dig", move |_world, _agent_id, _job_id, job| {
                let mut job = job.clone();
                let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                if matches!(
                    state,
                    "failed" | "complete" | "cancelled" | "interrupted" | "paused"
                ) {
                    return job;
                }
                let mut progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
                progress += 1.0;
                job["progress"] = json!(progress);
                if progress >= 3.0 {
                    job["state"] = json!("complete");
                } else {
                    job["state"] = json!("in_progress");
                }
                job
            });
    }

    // Agent and job setup
    let agent_id = world.spawn_entity();
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

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "progress": 0.0
            }),
        )
        .unwrap();

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

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
    for _ in 0..10 {
        // Increase to 10 ticks to guarantee completion
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
            .any(|e| e.get("entity") == Some(&json!(job_id))),
        "No job_assigned event for job"
    );
    // Check that job_completed event was emitted
    assert!(
        completed_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(job_id))),
        "No job_completed event for job"
    );
    // Check that at least one progress event was emitted
    assert!(
        progress_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(job_id))),
        "No job_progressed event for job"
    );

    // Check event payloads are rich and consistent
    for event in assigned_events
        .iter()
        .chain(completed_events.iter())
        .chain(progress_events.iter())
    {
        assert!(event.get("entity").is_some());
        assert!(event.get("job_type").is_some());
        assert!(event.get("state").is_some());
    }
}

#[test]
fn test_job_cancel_and_failure_events() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Setup a job that will fail
    let fail_job_id = world.spawn_entity();
    world
        .set_component(
            fail_job_id,
            "Job",
            json!({
                "id": fail_job_id,
                "job_type": "failtest",
                "state": "pending",
                "should_fail": true,
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();

    // Setup a job that will be cancelled
    let cancel_job_id = world.spawn_entity();
    world
        .set_component(
            cancel_job_id,
            "Job",
            json!({
                "id": cancel_job_id,
                "job_type": "dig",
                "state": "cancelled",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

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
            .any(|e| e.get("entity") == Some(&json!(fail_job_id))),
        "No job_failed event for failed job"
    );
    assert!(
        cancelled_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(cancel_job_id))),
        "No job_cancelled event for cancelled job"
    );
}
