#[path = "helpers/event.rs"]
mod event_helper;
#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::system::JobSystem;
use serde_json::json;

#[test]
fn test_job_progressed_event_emitted_on_progress_change() {
    let mut world = world_helper::make_test_world();

    // Set up event capture for job_progressed
    let capture = event_helper::setup_event_capture(&mut world, "job_progressed");

    // Agent with moderate skill
    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "skills": { "dig": 2.0 },
                "stamina": 100.0,
                "state": "working",
                "current_job": 10
            }),
        )
        .unwrap();
    world.entities.push(1);

    // Job assigned to agent
    world
        .set_component(
            10,
            "Job",
            json!({
                "id": 10,
                "job_type": "dig",
                "progress": 0.0,
                "status": "in_progress",
                "assigned_to": 1,
                "category": "mining",
                "phase": "in_progress"
            }),
        )
        .unwrap();
    world.entities.push(10);

    let mut job_system = JobSystem::new();

    // Run system for two ticks to ensure progress and event emission
    job_system.run(&mut world, None);
    job_system.run(&mut world, None);

    let event = capture
        .buffer
        .lock()
        .unwrap()
        .pop_front()
        .expect("No event emitted!");
    assert_eq!(
        event.get("entity").and_then(|v| v.as_u64()).unwrap(),
        10,
        "Job entity id should match"
    );
    assert_eq!(
        event.get("job_type").and_then(|v| v.as_str()).unwrap(),
        "dig",
        "Job type should match"
    );
    assert!(
        event.get("progress").is_some(),
        "Progress should be present in event"
    );
}

#[test]
fn test_job_progressed_event_emitted_for_custom_handler() {
    let mut world = world_helper::make_test_world();

    // Set up event capture for job_progressed
    let capture = event_helper::setup_event_capture(&mut world, "job_progressed");

    // Register a custom handler for "superfast" job_type
    world.register_job_handler(
        "superfast",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            job["progress"] = serde_json::json!(999.0);
            job["status"] = serde_json::json!("complete");
            job
        },
    );

    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "state": "working",
                "current_job": 123
            }),
        )
        .unwrap();
    world.entities.push(1);

    world
        .set_component(
            123,
            "Job",
            json!({
                "id": 123,
                "job_type": "superfast",
                "status": "in_progress",
                "progress": 0.0,
                "assigned_to": 1,
                "category": "testing",
                "phase": "in_progress"
            }),
        )
        .unwrap();
    world.entities.push(123);

    let mut job_system = JobSystem::new();

    // Run system for two ticks to ensure handler is invoked and event is emitted
    job_system.run(&mut world, None);
    job_system.run(&mut world, None);

    let event = capture
        .buffer
        .lock()
        .unwrap()
        .pop_front()
        .expect("No event emitted!");
    assert_eq!(
        event.get("entity").and_then(|v| v.as_u64()).unwrap(),
        123,
        "Job entity id should match"
    );
    assert_eq!(
        event.get("job_type").and_then(|v| v.as_str()).unwrap(),
        "superfast",
        "Job type should match"
    );
    assert_eq!(
        event.get("progress").and_then(|v| v.as_f64()).unwrap(),
        999.0,
        "Progress should match value set by custom handler"
    );
}
