#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, assign_jobs};
use serde_json::json;

#[test]
fn test_agent_state_and_job_completion() {
    let mut world = world_helper::make_test_world();

    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "skills": { "dig": 5.0 },
                "preferences": { "dig": 2.0 },
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
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(100);

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    let job = world.get_component(100, "Job").unwrap();

    assert_eq!(
        agent["current_job"], 100,
        "Agent should be assigned job 100"
    );
    assert_eq!(agent["state"], "working", "Agent should be working");
    assert_eq!(
        job["assigned_to"], 1,
        "Job 100 should be assigned to agent 1"
    );

    // Let the system process the job to completion
    let mut job_system = JobSystem;
    for _ in 0..5 {
        job_system.run(&mut world, None);
        let job = world.get_component(100, "Job").unwrap();
        if job["status"] == "complete" {
            break;
        }
    }

    let agent = world.get_component(1, "Agent").unwrap();
    assert!(
        agent.get("current_job").is_none(),
        "Agent should have no current job after completion"
    );
    assert_eq!(
        agent["state"], "idle",
        "Agent should be idle after completion"
    );
}

#[test]
fn test_job_preemption_and_reassignment() {
    let mut world = world_helper::make_test_world();

    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "skills": { "dig": 5.0, "build": 1.0 },
                "preferences": { "dig": 2.0 },
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(1);

    // Low priority job assigned first
    world
        .set_component(
            100,
            "Job",
            json!({
                "id": 100,
                "job_type": "dig",
                "status": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(100);

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    assign_jobs(&mut world, &mut job_board);

    {
        let agent = world.get_component(1, "Agent").unwrap();
        let job = world.get_component(100, "Job").unwrap();
        assert_eq!(
            agent["current_job"], 100,
            "Agent should be assigned job 100"
        );
        assert_eq!(agent["state"], "working", "Agent should be working");
        assert_eq!(
            job["assigned_to"], 1,
            "Job 100 should be assigned to agent 1"
        );
    }

    // Now, a higher-priority job appears
    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "dig",
                "status": "pending",
                "priority": 10,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(200);

    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    {
        let agent = world.get_component(1, "Agent").unwrap();
        let job100 = world.get_component(100, "Job").unwrap();
        let job200 = world.get_component(200, "Job").unwrap();

        // The agent should now be working on job 200 (preemption)
        assert_eq!(
            agent["current_job"], 200,
            "Agent should be assigned job 200 (preemption)"
        );
        assert_eq!(agent["state"], "working", "Agent should be working");
        assert_eq!(
            job200["assigned_to"], 1,
            "Job 200 should be assigned to agent 1"
        );

        // Job 100 should be unassigned and set to pending
        assert!(
            job100.get("assigned_to").is_none(),
            "Job 100 should be unassigned"
        );
        assert_eq!(job100["status"], "pending", "Job 100 should be pending");
    }

    // Simulate job 200 completion and agent becoming idle
    {
        let mut job = world.get_component(200, "Job").unwrap().clone();
        job["status"] = json!("complete");
        world.set_component(200, "Job", job).unwrap();
    }

    {
        let mut agent = world.get_component(1, "Agent").unwrap().clone();
        agent.as_object_mut().unwrap().remove("current_job");
        agent["state"] = json!("idle");
        world.set_component(1, "Agent", agent).unwrap();
    }

    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    {
        let agent = world.get_component(1, "Agent").unwrap();
        let job100 = world.get_component(100, "Job").unwrap();

        // The agent should now be reassigned to job 100
        assert_eq!(
            agent["current_job"], 100,
            "Agent should be reassigned to job 100"
        );
        assert_eq!(agent["state"], "working", "Agent should be working");
        assert_eq!(
            job100["assigned_to"], 1,
            "Job 100 should be assigned to agent 1"
        );
    }
}
