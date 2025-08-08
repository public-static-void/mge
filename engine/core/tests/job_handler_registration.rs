#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, assign_jobs};
use serde_json::json;

#[test]
fn test_register_job_handler_api_invokes_handler() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

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
                "job_type": "superfast",
                "state": "pending",
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    // Run the job system, custom handler should immediately complete the job
    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let job = world.get_component(job_id, "Job").unwrap();
    assert_eq!(job.get("progress").unwrap(), 999.0);
    assert_eq!(job.get("state").unwrap(), "complete");
}

#[test]
fn test_register_job_handler_multiple_types() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register handlers for two types
    world.register_job_handler(
        "foo",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            job["progress"] = serde_json::json!(1.0);
            job["state"] = serde_json::json!("complete");
            job
        },
    );
    world.register_job_handler(
        "bar",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            job["progress"] = serde_json::json!(2.0);
            job["state"] = serde_json::json!("complete");
            job
        },
    );

    // Add jobs of both types
    let job_foo_id = world.spawn_entity();
    world
        .set_component(
            job_foo_id,
            "Job",
            json!({
                "id": job_foo_id,
                "job_type": "foo",
                "state": "pending",
                "priority": 1,
                "category": "foo"
            }),
        )
        .unwrap();

    let job_bar_id = world.spawn_entity();
    world
        .set_component(
            job_bar_id,
            "Job",
            json!({
                "id": job_bar_id,
                "job_type": "bar",
                "state": "pending",
                "priority": 1,
                "category": "bar"
            }),
        )
        .unwrap();

    // Add two agents so both jobs can be assigned
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

    let agent2_id = world.spawn_entity();
    world
        .set_component(
            agent2_id,
            "Agent",
            json!({
                "entity_id": agent2_id,
                "state": "idle"
            }),
        )
        .unwrap();

    // Assign jobs to agents
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    // Run the job system
    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let job_foo = world.get_component(job_foo_id, "Job").unwrap();
    let job_bar = world.get_component(job_bar_id, "Job").unwrap();
    assert_eq!(job_foo.get("progress").unwrap(), 1.0);
    assert_eq!(job_foo.get("state").unwrap(), "complete");
    assert_eq!(job_bar.get("progress").unwrap(), 2.0);
    assert_eq!(job_bar.get("state").unwrap(), "complete");
}
