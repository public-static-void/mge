#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, assign_jobs};
use serde_json::json;

/// Tests that an agent is assigned a job, completes it, and becomes idle.
#[test]
fn test_agent_state_and_job_completion() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "dig": 5.0 },
                "preferences": { "dig": 2.0 },
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
                "category": "mining"
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);

    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent = world.get_component(agent_id, "Agent").unwrap();
    let job = world.get_component(job_id, "Job").unwrap();

    assert_eq!(agent["current_job"], job_id, "Agent should be assigned job");
    assert_eq!(agent["state"], "working", "Agent should be working");
    assert_eq!(
        job["assigned_to"], agent_id,
        "Job should be assigned to agent"
    );

    // Let the system process the job to completion
    let mut job_system = JobSystem;
    for _ in 0..5 {
        job_system.run(&mut world, None);
        let job = world.get_component(job_id, "Job").unwrap();
        if job["state"] == "complete" {
            break;
        }
    }

    let agent = world.get_component(agent_id, "Agent").unwrap();
    assert_eq!(
        agent.get("current_job"),
        Some(&serde_json::Value::Null),
        "Agent should have no current job after completion"
    );
    assert_eq!(
        agent["state"], "idle",
        "Agent should be idle after completion"
    );
}

/// Tests that a higher-priority job preempts a lower-priority job, and that the agent is reassigned after completion.
#[test]
fn test_job_preemption_and_reassignment() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "dig": 1.0, "build": 1.0 },
                "preferences": { "dig": 2.0 },
                "state": "idle"
            }),
        )
        .unwrap();

    // Low priority job assigned first
    let job100 = world.spawn_entity();
    world
        .set_component(
            job100,
            "Job",
            json!({
                "id": job100,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "required_progress": 10.0
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);

    assign_jobs(&mut world, &mut job_board, 0, &[]);

    {
        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job = world.get_component(job100, "Job").unwrap();
        assert_eq!(
            agent["current_job"], job100,
            "Agent should be assigned job 100"
        );
        assert_eq!(agent["state"], "working", "Agent should be working");
        assert_eq!(
            job["assigned_to"], agent_id,
            "Job 100 should be assigned to agent"
        );
    }

    // Now, a higher-priority job appears
    let job200 = world.spawn_entity();
    world
        .set_component(
            job200,
            "Job",
            json!({
                "id": job200,
                "job_type": "dig",
                "state":"pending",
                "priority": 10,
                "category": "mining"
            }),
        )
        .unwrap();

    // Run the job system and assign_jobs in a loop until preemption occurs
    let mut preempted = false;
    for _ in 0..10 {
        job_board.update(&world, 0, &[]);
        assign_jobs(&mut world, &mut job_board, 0, &[]);

        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job100_obj = world.get_component(job100, "Job").unwrap();
        let job200_obj = world.get_component(job200, "Job").unwrap();

        if agent["current_job"] == job200 {
            preempted = true;
            // The agent should now be working on job 200 (preemption)
            assert_eq!(
                agent["current_job"], job200,
                "Agent should be assigned job 200 (preemption)"
            );
            assert_eq!(agent["state"], "working", "Agent should be working");
            assert_eq!(
                job200_obj["assigned_to"], agent_id,
                "Job 200 should be assigned to agent"
            );

            // Job 100 should be unassigned and set to pending
            assert!(
                job100_obj.get("assigned_to") == Some(&serde_json::Value::Null),
                "Job 100 should be unassigned"
            );
            assert_eq!(job100_obj["state"], "pending", "Job 100 should be pending");
            break;
        }

        let mut job_system = JobSystem;
        job_system.run(&mut world, None);
    }

    assert!(preempted, "Agent was never preempted to job 200");

    // Simulate job 200 completion and agent becoming idle
    {
        let mut job = world.get_component(job200, "Job").unwrap().clone();
        job["state"] = json!("complete");
        world.set_component(job200, "Job", job).unwrap();
    }

    {
        let mut agent = world.get_component(agent_id, "Agent").unwrap().clone();
        agent.as_object_mut().unwrap().remove("current_job");
        agent["state"] = json!("idle");
        world.set_component(agent_id, "Agent", agent).unwrap();
    }

    // --- Repeatedly assign jobs and run the job system until job 100 is reassigned ---
    loop {
        job_board.update(&world, 0, &[]);
        assign_jobs(&mut world, &mut job_board, 0, &[]);
        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job100_obj = world.get_component(job100, "Job").unwrap();
        if agent["current_job"] == job100 && job100_obj["assigned_to"] == agent_id {
            break;
        }
        let mut job_system = JobSystem;
        job_system.run(&mut world, None);
    }

    {
        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job100_obj = world.get_component(job100, "Job").unwrap();

        // The agent should now be reassigned to job 100
        assert_eq!(
            agent["current_job"], job100,
            "Agent should be reassigned to job 100"
        );
        assert_eq!(agent["state"], "working", "Agent should be working");
        assert_eq!(
            job100_obj["assigned_to"], agent_id,
            "Job 100 should be assigned to agent"
        );
    }
}
