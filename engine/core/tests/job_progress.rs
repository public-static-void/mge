#[path = "helpers/event.rs"]
mod event_helper;
#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::system::JobSystem;
use serde_json::json;

#[test]
fn test_job_progressed_event_emitted_on_progress_change() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Set up event capture for job_progressed
    let capture = event_helper::setup_event_capture(&mut world, "job_progressed");

    // Agent with moderate skill
    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "dig": 2.0 },
                "stamina": 100.0,
                "state": "working",
                "current_job": job_id
            }),
        )
        .unwrap();

    // Job assigned to agent
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

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
        job_id as u64,
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
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Set up event capture for job_progressed
    let capture = event_helper::setup_event_capture(&mut world, "job_progressed");

    // Register a custom handler for "superfast" job_type
    world.register_job_handler(
        "superfast",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            job["progress"] = serde_json::json!(999.0);
            job["state"] = serde_json::json!("complete");
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
                "state": "working",
                "current_job": job_id
            }),
        )
        .unwrap();

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "superfast",
                "state": "in_progress",
                "progress": 0.0,
                "assigned_to": agent_id,
                "category": "testing",
            }),
        )
        .unwrap();

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
        job_id as u64,
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
