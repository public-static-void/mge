#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::assign_jobs;
use engine_core::systems::job::job_board::JobBoard;
use serde_json::json;

#[test]
fn test_ai_job_assignment_priority_and_state() {
    let mut world = world_helper::make_test_world();

    // Agent 1: idle, high dig skill
    let agent1 = world.spawn_entity();
    world
        .set_component(
            agent1,
            "Agent",
            json!({
                "entity_id": agent1,
                "skills": { "dig": 5.0, "build": 1.0 },
                "preferences": { "dig": 2.0 },
                "state": "idle",
                "category": "mining"
            }),
        )
        .unwrap();

    // Agent 2: idle, high build skill
    let agent2 = world.spawn_entity();
    world
        .set_component(
            agent2,
            "Agent",
            json!({
                "entity_id": agent2,
                "skills": { "dig": 0.0, "build": 6.0 },
                "preferences": { "build": 2.0 },
                "state": "idle",
                "category": "mining"
            }),
        )
        .unwrap();

    // Agent 3: not idle, should not get a job
    let agent3 = world.spawn_entity();
    world
        .set_component(
            agent3,
            "Agent",
            json!({
                "entity_id": agent3,
                "skills": { "dig": 10.0 },
                "preferences": { "dig": 10.0 },
                "state": "working",
                "category": "mining"
            }),
        )
        .unwrap();

    // Job 100: dig, priority 1
    let job100 = world.spawn_entity();
    world
        .set_component(
            job100,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Job 200: build, priority 5
    let job200 = world.spawn_entity();
    world
        .set_component(
            job200,
            "Job",
            json!({
                "job_type": "build",
                "status": "pending",
                "priority": 5,
                "category": "construction"
            }),
        )
        .unwrap();

    // Job 300: dig, priority 10
    let job300 = world.spawn_entity();
    world
        .set_component(
            job300,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending",
                "priority": 10,
                "category": "construction"
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    assign_jobs(&mut world, &mut job_board);

    {
        let agent1_obj = world.get_component(agent1, "Agent").unwrap();
        let agent2_obj = world.get_component(agent2, "Agent").unwrap();
        let agent3_obj = world.get_component(agent3, "Agent").unwrap();
        let job100_obj = world.get_component(job100, "Job").unwrap();
        let job200_obj = world.get_component(job200, "Job").unwrap();
        let job300_obj = world.get_component(job300, "Job").unwrap();

        // Agent 1 should get highest priority dig job (job 300)
        assert_eq!(agent1_obj["current_job"], job300);
        assert_eq!(agent1_obj["state"], "working");
        assert_eq!(job300_obj["assigned_to"], agent1);

        // Agent 2 should get highest priority build job (job 200)
        assert_eq!(agent2_obj["current_job"], job200);
        assert_eq!(agent2_obj["state"], "working");
        assert_eq!(job200_obj["assigned_to"], agent2);

        // Agent 3 should remain unchanged (not idle)
        assert!(agent3_obj.get("current_job").is_none());
        assert_eq!(agent3_obj["state"], "working");

        // Job 100 should remain unassigned (no idle agent left)
        assert!(job100_obj.get("assigned_to").is_none());
    }
}

#[test]
fn test_agent_job_queue_and_resource_aware_assignment() {
    let mut world = world_helper::make_test_world();

    let stockpile = world.spawn_entity();
    world
        .set_component(
            stockpile,
            "Stockpile",
            json!({ "resources": { "wood": 0 } }),
        )
        .unwrap();

    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "skills": { "dig": 2.0, "build": 1.0 },
                "preferences": { "dig": 1.0 },
                "state": "idle",
                "job_queue": [ ]
            }),
        )
        .unwrap();

    let job100 = world.spawn_entity();
    world
        .set_component(
            job100,
            "Job",
            json!({
                "id": job100,
                "job_type": "dig",
                "status": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    let job200 = world.spawn_entity();
    world
        .set_component(
            job200,
            "Job",
            json!({
                "id": job200,
                "job_type": "build",
                "status": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "wood", "amount": 5 } ],
                "category": "construction"
            }),
        )
        .unwrap();

    // Add jobs to agent's queue
    {
        let mut agent_obj = world.get_component(agent, "Agent").unwrap().clone();
        agent_obj["job_queue"] = json!([job100, job200]);
        world.set_component(agent, "Agent", agent_obj).unwrap();
    }

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    assign_jobs(&mut world, &mut job_board);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(agent_obj["current_job"], job100);

    // Simulate job 100 completion
    let mut job = world.get_component(job100, "Job").unwrap().clone();
    job["status"] = json!("complete");
    world.set_component(job100, "Job", job).unwrap();

    let mut agent_obj = world.get_component(agent, "Agent").unwrap().clone();
    agent_obj.as_object_mut().unwrap().remove("current_job");
    agent_obj["state"] = json!("idle");
    world.set_component(agent, "Agent", agent_obj).unwrap();

    assign_jobs(&mut world, &mut job_board);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(agent_obj["current_job"], job200);
}

#[test]
fn test_job_preemption_by_higher_priority() {
    let mut world = world_helper::make_test_world();

    // Agent: idle, can do both jobs
    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "skills": { "dig": 5.0, "build": 5.0 },
                "preferences": { "dig": 2.0, "build": 2.0 },
                "state": "idle"
            }),
        )
        .unwrap();

    let job100 = world.spawn_entity();
    world
        .set_component(
            job100,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Assign initial job
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(agent_obj["current_job"], job100);
    assert_eq!(agent_obj["state"], "working");

    // Simulate job 100 in progress
    let mut job = world.get_component(job100, "Job").unwrap().clone();
    job["status"] = json!("in_progress");
    world.set_component(job100, "Job", job).unwrap();

    // Now, add a higher-priority job
    let job200 = world.spawn_entity();
    world
        .set_component(
            job200,
            "Job",
            json!({
                "job_type": "build",
                "status": "pending",
                "priority": 10,
                "category": "construction"
            }),
        )
        .unwrap();

    // Run assignment again: agent should preempt job 100 for job 200
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(
        agent_obj["current_job"], job200,
        "Agent should preempt to higher-priority job"
    );
    assert_eq!(agent_obj["state"], "working");

    let job100_obj = world.get_component(job100, "Job").unwrap();
    assert!(
        job100_obj.get("assigned_to").is_none() || job100_obj["assigned_to"] != agent,
        "Old job should be unassigned"
    );
    let job200_obj = world.get_component(job200, "Job").unwrap();
    assert_eq!(job200_obj["assigned_to"], agent);
}

#[test]
fn test_agent_abandons_job_if_blocked() {
    let mut world = world_helper::make_test_world();

    // Agent: working on job 100
    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "skills": { "dig": 5.0 },
                "preferences": { "dig": 2.0 },
                "state": "working",
                "current_job": 0 // will be set below
            }),
        )
        .unwrap();

    let job100 = world.spawn_entity();
    world
        .set_component(
            job100,
            "Job",
            json!({
                "job_type": "dig",
                "status": "in_progress",
                "assigned_to": agent,
                "blocked": true,
                "category": "mining"
            }),
        )
        .unwrap();

    // Update agent's current_job to point to job100
    {
        let mut agent_obj = world.get_component(agent, "Agent").unwrap().clone();
        agent_obj["current_job"] = json!(job100);
        world.set_component(agent, "Agent", agent_obj).unwrap();
    }

    // Run assignment: agent should abandon blocked job and become idle
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert!(
        agent_obj.get("current_job").is_none(),
        "Agent should abandon blocked job"
    );
    assert_eq!(agent_obj["state"], "idle");
    let job_obj = world.get_component(job100, "Job").unwrap();
    assert!(
        job_obj.get("assigned_to").is_none(),
        "Blocked job should be unassigned"
    );
}

#[test]
fn test_dynamic_priority_update_affects_assignment() {
    let mut world = world_helper::make_test_world();

    // Agent: idle, can do both jobs
    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "skills": { "dig": 5.0, "build": 5.0 },
                "preferences": { "dig": 2.0, "build": 2.0 },
                "state": "idle"
            }),
        )
        .unwrap();

    let job100 = world.spawn_entity();
    world
        .set_component(
            job100,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    let job200 = world.spawn_entity();
    world
        .set_component(
            job200,
            "Job",
            json!({
                "job_type": "build",
                "status": "pending",
                "priority": 5,
                "category": "construction"
            }),
        )
        .unwrap();

    // Initial assignment: agent should pick job 200 (higher priority)
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(agent_obj["current_job"], job200);

    // Now, increase priority of job 100 and re-run assignment
    let mut job = world.get_component(job100, "Job").unwrap().clone();
    job["priority"] = json!(10);
    world.set_component(job100, "Job", job).unwrap();

    // Unassign agent from job 200
    let mut job = world.get_component(job200, "Job").unwrap().clone();
    job.as_object_mut().unwrap().remove("assigned_to");
    world.set_component(job200, "Job", job).unwrap();

    let mut agent_obj = world.get_component(agent, "Agent").unwrap().clone();
    agent_obj.as_object_mut().unwrap().remove("current_job");
    agent_obj["state"] = json!("idle");
    world.set_component(agent, "Agent", agent_obj).unwrap();

    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(
        agent_obj["current_job"], job100,
        "Agent should now have the higher-priority job"
    );
}
