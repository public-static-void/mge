#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, assign_jobs};
use serde_json::json;

#[test]
fn test_register_job_handler_api_invokes_handler() {
    let mut world = world_helper::make_test_world();

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
                "state": "idle"
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
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();
    world.entities.push(123);

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    // Run the job system, custom handler should immediately complete the job
    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let job = world.get_component(123, "Job").unwrap();
    assert_eq!(job.get("progress").unwrap(), 999.0);
    assert_eq!(job.get("status").unwrap(), "complete");
}

#[test]
fn test_register_job_handler_multiple_types() {
    let mut world = world_helper::make_test_world();

    // Register handlers for two types
    world.register_job_handler(
        "foo",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            job["progress"] = serde_json::json!(1.0);
            job["status"] = serde_json::json!("complete");
            job
        },
    );
    world.register_job_handler(
        "bar",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            job["progress"] = serde_json::json!(2.0);
            job["status"] = serde_json::json!("complete");
            job
        },
    );

    // Add jobs of both types
    world
        .set_component(
            10,
            "Job",
            json!({
                "id": 10,
                "job_type": "foo",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "foo"
            }),
        )
        .unwrap();
    world.entities.push(10);

    world
        .set_component(
            20,
            "Job",
            json!({
                "id": 20,
                "job_type": "bar",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "bar"
            }),
        )
        .unwrap();
    world.entities.push(20);

    // Add an agent so jobs can be assigned
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

    // Assign jobs to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    // Run the job system
    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let job_foo = world.get_component(10, "Job").unwrap();
    let job_bar = world.get_component(20, "Job").unwrap();
    assert_eq!(job_foo.get("progress").unwrap(), 1.0);
    assert_eq!(job_foo.get("status").unwrap(), "complete");
    assert_eq!(job_bar.get("progress").unwrap(), 2.0);
    assert_eq!(job_bar.get("status").unwrap(), "complete");
}
