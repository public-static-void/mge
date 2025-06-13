use engine_core::ecs::world::World;
use engine_core::systems::job::assign_jobs;
use engine_core::systems::job_board::JobBoard;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn setup_registry() -> Arc<Mutex<engine_core::ecs::registry::ComponentRegistry>> {
    let mut registry = engine_core::ecs::registry::ComponentRegistry::default();
    registry.register_external_schema(engine_core::ecs::schema::ComponentSchema {
        name: "Agent".to_string(),
        schema: serde_json::json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    registry.register_external_schema(engine_core::ecs::schema::ComponentSchema {
        name: "Job".to_string(),
        schema: serde_json::json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    registry.register_external_schema(engine_core::ecs::schema::ComponentSchema {
        name: "Stockpile".to_string(),
        schema: serde_json::json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    Arc::new(Mutex::new(registry))
}

#[test]
fn test_ai_job_assignment_priority_and_state() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Agent 1: idle, high dig skill
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

    // Agent 2: idle, high build skill
    world
        .set_component(
            2,
            "Agent",
            json!({
                "entity_id": 2,
                "skills": { "dig": 0.0, "build": 6.0 },
                "preferences": { "build": 2.0 },
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(2);

    // Agent 3: not idle, should not get a job
    world
        .set_component(
            3,
            "Agent",
            json!({
                "entity_id": 3,
                "skills": { "dig": 10.0 },
                "preferences": { "dig": 10.0 },
                "state": "working"
            }),
        )
        .unwrap();
    world.entities.push(3);

    // Job 100: dig, priority 1
    world
        .set_component(
            100,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending",
                "priority": 1
            }),
        )
        .unwrap();
    world.entities.push(100);

    // Job 200: build, priority 5
    world
        .set_component(
            200,
            "Job",
            json!({
                "job_type": "build",
                "status": "pending",
                "priority": 5
            }),
        )
        .unwrap();
    world.entities.push(200);

    // Job 300: dig, priority 10
    world
        .set_component(
            300,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending",
                "priority": 10
            }),
        )
        .unwrap();
    world.entities.push(300);

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    assign_jobs(&mut world, &mut job_board);

    {
        let agent1 = world.get_component(1, "Agent").unwrap();
        let agent2 = world.get_component(2, "Agent").unwrap();
        let agent3 = world.get_component(3, "Agent").unwrap();
        let job100 = world.get_component(100, "Job").unwrap();
        let job200 = world.get_component(200, "Job").unwrap();
        let job300 = world.get_component(300, "Job").unwrap();

        // Agent 1 should get highest priority dig job (job 300)
        assert_eq!(agent1["current_job"], 300);
        assert_eq!(agent1["state"], "working");
        assert_eq!(job300["assigned_to"], 1);

        // Agent 2 should get highest priority build job (job 200)
        assert_eq!(agent2["current_job"], 200);
        assert_eq!(agent2["state"], "working");
        assert_eq!(job200["assigned_to"], 2);

        // Agent 3 should remain unchanged (not idle)
        assert!(agent3.get("current_job").is_none());
        assert_eq!(agent3["state"], "working");

        // Job 100 should remain unassigned (no idle agent left)
        assert!(job100.get("assigned_to").is_none());
    }
}

#[test]
fn test_agent_job_queue_and_resource_aware_assignment() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    world
        .set_component(10, "Stockpile", json!({ "resources": { "wood": 0 } }))
        .unwrap();
    world.entities.push(10);

    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "skills": { "dig": 2.0, "build": 1.0 },
                "preferences": { "dig": 1.0 },
                "state": "idle",
                "job_queue": [100, 200]
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
                "priority": 1
            }),
        )
        .unwrap();
    world.entities.push(100);

    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "build",
                "status": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "wood", "amount": 5 } ]
            }),
        )
        .unwrap();
    world.entities.push(200);

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    assert_eq!(agent["current_job"], 100);

    // Simulate job 100 completion
    let mut job = world.get_component(100, "Job").unwrap().clone();
    job["status"] = json!("complete");
    world.set_component(100, "Job", job).unwrap();

    let mut agent = world.get_component(1, "Agent").unwrap().clone();
    agent.as_object_mut().unwrap().remove("current_job");
    agent["state"] = json!("idle");
    world.set_component(1, "Agent", agent).unwrap();

    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    assert_eq!(agent["current_job"], 200);
}

#[test]
fn test_job_preemption_by_higher_priority() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Agent: idle, can do both jobs
    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "skills": { "dig": 5.0, "build": 5.0 },
                "preferences": { "dig": 2.0, "build": 2.0 },
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(1);

    // Job 100: dig, priority 1
    world
        .set_component(
            100,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending",
                "priority": 1
            }),
        )
        .unwrap();
    world.entities.push(100);

    // Assign initial job
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    assert_eq!(agent["current_job"], 100);
    assert_eq!(agent["state"], "working");

    // Simulate job 100 in progress
    let mut job = world.get_component(100, "Job").unwrap().clone();
    job["status"] = json!("in_progress");
    world.set_component(100, "Job", job).unwrap();

    // Now, add a higher-priority job
    world
        .set_component(
            200,
            "Job",
            json!({
                "job_type": "build",
                "status": "pending",
                "priority": 10
            }),
        )
        .unwrap();
    world.entities.push(200);

    // Run assignment again: agent should preempt job 100 for job 200
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    assert_eq!(
        agent["current_job"], 200,
        "Agent should preempt to higher-priority job"
    );
    assert_eq!(agent["state"], "working");

    let job100 = world.get_component(100, "Job").unwrap();
    assert!(
        job100.get("assigned_to").is_none() || job100["assigned_to"] != 1,
        "Old job should be unassigned"
    );
    let job200 = world.get_component(200, "Job").unwrap();
    assert_eq!(job200["assigned_to"], 1);
}

#[test]
fn test_agent_abandons_job_if_blocked() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Agent: working on job 100
    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "skills": { "dig": 5.0 },
                "preferences": { "dig": 2.0 },
                "state": "working",
                "current_job": 100
            }),
        )
        .unwrap();
    world.entities.push(1);

    // Job 100: dig, assigned to agent, but now blocked (simulate with "blocked": true)
    world
        .set_component(
            100,
            "Job",
            json!({
                "job_type": "dig",
                "status": "in_progress",
                "assigned_to": 1,
                "blocked": true
            }),
        )
        .unwrap();
    world.entities.push(100);

    // Run assignment: agent should abandon blocked job and become idle
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    assert!(
        agent.get("current_job").is_none(),
        "Agent should abandon blocked job"
    );
    assert_eq!(agent["state"], "idle");
    let job = world.get_component(100, "Job").unwrap();
    assert!(
        job.get("assigned_to").is_none(),
        "Blocked job should be unassigned"
    );
}

#[test]
fn test_dynamic_priority_update_affects_assignment() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Agent: idle, can do both jobs
    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "skills": { "dig": 5.0, "build": 5.0 },
                "preferences": { "dig": 2.0, "build": 2.0 },
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(1);

    // Job 100: dig, priority 1
    world
        .set_component(
            100,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending",
                "priority": 1
            }),
        )
        .unwrap();
    world.entities.push(100);

    // Job 200: build, priority 5
    world
        .set_component(
            200,
            "Job",
            json!({
                "job_type": "build",
                "status": "pending",
                "priority": 5
            }),
        )
        .unwrap();
    world.entities.push(200);

    // Initial assignment: agent should pick job 200 (higher priority)
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    assert_eq!(agent["current_job"], 200);

    // Now, increase priority of job 100 and re-run assignment
    let mut job = world.get_component(100, "Job").unwrap().clone();
    job["priority"] = json!(10);
    world.set_component(100, "Job", job).unwrap();

    // Unassign agent from job 200
    let mut job = world.get_component(200, "Job").unwrap().clone();
    job.as_object_mut().unwrap().remove("assigned_to");
    world.set_component(200, "Job", job).unwrap();

    let mut agent = world.get_component(1, "Agent").unwrap().clone();
    agent.as_object_mut().unwrap().remove("current_job");
    agent["state"] = json!("idle");
    world.set_component(1, "Agent", agent).unwrap();

    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    assert_eq!(
        agent["current_job"], 100,
        "Agent should now have the higher-priority job"
    );
}
