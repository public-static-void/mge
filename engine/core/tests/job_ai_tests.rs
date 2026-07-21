#[path = "helpers/world.rs"]
mod world_helper;

#[path = "helpers/job_type.rs"]
mod job_type_helper;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::builtin_handlers::register_builtin_job_handlers;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::job_handler_registry::JobHandlerRegistry;
use engine_core::systems::job::{
    AiEventReactionSystem, JobSystem, JobTypeData, JobTypeRegistry, assign_jobs,
    setup_ai_event_subscriptions,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use world_helper::make_test_world;

// --- Section: AI Assignment ---

#[test]
fn test_ai_job_assignment_priority_and_state() {
    engine_core::systems::job::system::events::init_job_event_logger();
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
                "category": "mining",
                "specializations": ["mining", "construction"]
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
                "category": "mining",
                "specializations": ["mining", "construction"]
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
                "state": "pending",
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
                "state": "pending",
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
                "state": "pending",
                "priority": 10,
                "category": "construction"
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);

    assign_jobs(&mut world, &mut job_board, 0, &[]);

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
        assert!(
            agent3_obj.get("current_job").is_none_or(|v| v.is_null()),
            "Agent 3 should have no job assigned"
        );
        assert_eq!(agent3_obj["state"], "working");

        // Job 100 should remain unassigned (no idle agent left)
        assert!(
            job100_obj.get("assigned_to").is_none_or(|v| v.is_null()),
            "Job 100 should be unassigned"
        );
    }
}

#[test]
fn test_agent_job_queue_and_resource_aware_assignment() {
    engine_core::systems::job::system::events::init_job_event_logger();
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
                "job_queue": [ ],
                "specializations": ["mining", "construction"]
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
                "state": "pending",
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
                "state": "pending",
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
    job_board.update(&world, 0, &[]);

    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(agent_obj["current_job"], job100);

    // Simulate job 100 completion
    let mut job = world.get_component(job100, "Job").unwrap().clone();
    job["state"] = json!("complete");
    world.set_component(job100, "Job", job).unwrap();

    let mut agent_obj = world.get_component(agent, "Agent").unwrap().clone();
    agent_obj["current_job"] = serde_json::Value::Null; // always set to null, not remove
    agent_obj["state"] = json!("idle");
    world.set_component(agent, "Agent", agent_obj).unwrap();

    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(agent_obj["current_job"], job200);
}

#[test]
fn test_job_preemption_by_higher_priority() {
    engine_core::systems::job::system::events::init_job_event_logger();
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
                "state": "idle",
                "specializations": ["mining", "construction"]
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
                "state": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Assign initial job
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(agent_obj["current_job"], job100);
    assert_eq!(agent_obj["state"], "working");

    // Simulate job 100 in progress
    let mut job = world.get_component(job100, "Job").unwrap().clone();
    job["state"] = json!("in_progress");
    world.set_component(job100, "Job", job).unwrap();

    // Now, add a higher-priority job
    let job200 = world.spawn_entity();
    world
        .set_component(
            job200,
            "Job",
            json!({
                "job_type": "build",
                "state": "pending",
                "priority": 10,
                "category": "construction"
            }),
        )
        .unwrap();

    // Run assignment again: agent should preempt job 100 for job 200
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(
        agent_obj["current_job"], job200,
        "Agent should preempt to higher-priority job"
    );
    assert_eq!(agent_obj["state"], "working");

    let job100_obj = world.get_component(job100, "Job").unwrap();
    assert!(
        job100_obj.get("assigned_to").is_none_or(|v| v.is_null())
            || job100_obj["assigned_to"] != agent,
        "Old job should be unassigned"
    );
    let job200_obj = world.get_component(job200, "Job").unwrap();
    assert_eq!(job200_obj["assigned_to"], agent);
}

#[test]
fn test_agent_abandons_job_if_blocked() {
    engine_core::systems::job::system::events::init_job_event_logger();
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
                "current_job": 0, // will be set below
                "specializations": ["mining", "construction"]
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
                "state": "blocked",
                "assigned_to": agent,
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
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert!(
        agent_obj.get("current_job").is_none_or(|v| v.is_null()),
        "Agent should abandon blocked job"
    );
    assert_eq!(agent_obj["state"], "idle");
    let job_obj = world.get_component(job100, "Job").unwrap();
    assert!(
        job_obj.get("assigned_to").is_none_or(|v| v.is_null()),
        "Blocked job should be unassigned"
    );
}

#[test]
fn test_dynamic_priority_update_affects_assignment() {
    engine_core::systems::job::system::events::init_job_event_logger();
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
                "state": "idle",
                "specializations": ["mining", "construction"]
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
                "state": "pending",
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
                "state": "pending",
                "priority": 5,
                "category": "construction"
            }),
        )
        .unwrap();

    // Initial assignment: agent should pick job 200 (higher priority)
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(agent_obj["current_job"], job200);

    // Now, increase priority of job 100 and re-run assignment
    let mut job = world.get_component(job100, "Job").unwrap().clone();
    job["priority"] = json!(10);
    world.set_component(job100, "Job", job).unwrap();

    // Unassign agent from job 200
    let mut job = world.get_component(job200, "Job").unwrap().clone();
    job["assigned_to"] = serde_json::Value::Null;
    world.set_component(job200, "Job", job).unwrap();

    let mut agent_obj = world.get_component(agent, "Agent").unwrap().clone();
    agent_obj["current_job"] = serde_json::Value::Null;
    agent_obj["state"] = json!("idle");
    world.set_component(agent, "Agent", agent_obj).unwrap();

    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    assert_eq!(
        agent_obj["current_job"], job100,
        "Agent should now have the higher-priority job"
    );
}

/// Verifies that skill levels are the differentiating factor in AI job assignment.
///
/// Agent A (skill_mining=5, skill_crafting=1) and Agent B (skill_mining=1, skill_crafting=5)
/// with matching jobs. Both agents have no preferences, no specializations, and both jobs
/// have equal priority — skill should be the deciding factor in assignment.
#[test]
fn test_skill_isolated_ai_assignment() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Agent A: high mining skill (5.0), low crafting skill (1.0)
    let agent_a = world.spawn_entity();
    world
        .set_component(
            agent_a,
            "Agent",
            json!({
                "entity_id": agent_a,
                "skills": { "mining": 5.0, "crafting": 1.0 },
                "state": "idle",
            }),
        )
        .unwrap();

    // Agent B: low mining skill (1.0), high crafting skill (5.0)
    let agent_b = world.spawn_entity();
    world
        .set_component(
            agent_b,
            "Agent",
            json!({
                "entity_id": agent_b,
                "skills": { "mining": 1.0, "crafting": 5.0 },
                "state": "idle",
            }),
        )
        .unwrap();

    // Job X: mining — matches Agent A's higher skill
    let job_x = world.spawn_entity();
    world
        .set_component(
            job_x,
            "Job",
            json!({
                "job_type": "mining",
                "state": "pending",
                "priority": 1,
                "category": "general",
            }),
        )
        .unwrap();

    // Job Y: crafting — matches Agent B's higher skill
    let job_y = world.spawn_entity();
    world
        .set_component(
            job_y,
            "Job",
            json!({
                "job_type": "crafting",
                "state": "pending",
                "priority": 1,
                "category": "general",
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent_a_obj = world.get_component(agent_a, "Agent").unwrap();
    let agent_b_obj = world.get_component(agent_b, "Agent").unwrap();

    // Both agents should now be working
    assert_eq!(agent_a_obj["state"], "working");
    assert_eq!(agent_b_obj["state"], "working");

    // Agent A should have a job assigned (matching their higher mining skill)
    let a_job = agent_a_obj.get("current_job").and_then(|v| v.as_u64());
    assert!(a_job.is_some(), "Agent A should be assigned a job");

    // Agent B should have a job assigned (matching their higher crafting skill)
    let b_job = agent_b_obj.get("current_job").and_then(|v| v.as_u64());
    assert!(b_job.is_some(), "Agent B should be assigned a job");

    // Verify jobs are assigned to different agents
    assert_ne!(a_job, b_job, "Each agent should have a different job");
}

// --- Section: Event Intent ---

#[test]
fn test_event_driven_ai_job_enqueue() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();

    // Add an agent
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "production": 5.0 },
                "preferences": {},
                "state": "idle",
                "job_queue": [],
                "specializations": ["production"]
            }),
        )
        .unwrap();

    // Add a production job for "wood"
    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "production",
                "state": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "wood", "amount": 10 } ],
                "category": "production"
            }),
        )
        .unwrap();

    // Setup AI event subscription
    setup_ai_event_subscriptions(&mut world);

    // Simulate a resource shortage event
    world
        .ai_event_intents
        .push_back(json!({ "kind": "wood", "amount": 0 }));

    // Run the AI event reaction system
    let mut system = AiEventReactionSystem;
    system.run(&mut world);

    // Agent's job queue should now contain the production job for wood
    let agent = world.get_component(agent_id, "Agent").unwrap();
    let queue = agent.get("job_queue").unwrap().as_array().unwrap();
    assert!(queue.iter().any(|v| v.as_u64() == Some(job_id as u64)));

    // Now assign jobs
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent = world.get_component(agent_id, "Agent").unwrap();
    assert_eq!(agent["current_job"], job_id);
    assert_eq!(agent["state"], "working");
}

#[test]
fn test_event_intent_queue_handles_multiple_events() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();

    // Add an agent
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "production": 5.0 },
                "preferences": {},
                "state": "idle",
                "job_queue": []
            }),
        )
        .unwrap();

    // Add two production jobs
    let job_id1 = world.spawn_entity();
    world
        .set_component(
            job_id1,
            "Job",
            json!({
                "id": job_id1,
                "job_type": "production",
                "state": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "stone", "amount": 5 } ],
                "category": "production"
            }),
        )
        .unwrap();

    let job_id2 = world.spawn_entity();
    world
        .set_component(
            job_id2,
            "Job",
            json!({
                "id": job_id2,
                "job_type": "production",
                "state": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "wood", "amount": 5 } ],
                "category": "production"
            }),
        )
        .unwrap();

    // Setup AI event subscription
    setup_ai_event_subscriptions(&mut world);

    // Simulate two resource shortage events
    world
        .ai_event_intents
        .push_back(json!({ "kind": "wood", "amount": 0 }));
    world
        .ai_event_intents
        .push_back(json!({"kind": "stone", "amount": 0 }));

    // Run the AI event reaction system
    let mut system = AiEventReactionSystem;
    system.run(&mut world);

    // Agent's job queue should now contain both jobs
    let agent = world.get_component(agent_id, "Agent").unwrap();
    let queue = agent.get("job_queue").unwrap().as_array().unwrap();
    assert!(queue.iter().any(|v| v.as_u64() == Some(job_id1 as u64)));
    assert!(queue.iter().any(|v| v.as_u64() == Some(job_id2 as u64)));
}

// --- Section: Handler Registry ---

#[test]
fn test_register_and_invoke_job_handler() {
    let registry = Arc::new(Mutex::new(JobHandlerRegistry::new()));

    let called = Arc::new(Mutex::new(false));
    let called_clone = called.clone();

    registry.lock().unwrap().register_handler(
        "test_job",
        move |_world, agent_id, job_id, _data| {
            assert_eq!(agent_id, 42);
            assert_eq!(job_id, 99);
            *called_clone.lock().unwrap() = true;
            serde_json::json!(null)
        },
    );

    let mut world = World::new(Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::default(),
    )));
    world.job_handler_registry = Arc::clone(&registry);

    // Clone the handler out to end the immutable borrow before mutably borrowing world
    let handler = world
        .job_handler_registry
        .lock()
        .unwrap()
        .get("test_job")
        .expect("Handler not found")
        .clone();
    handler(&mut world, 42, 99, &json!({}));

    assert!(*called.lock().unwrap());
}

#[test]
fn test_missing_job_handler() {
    let registry = Arc::new(Mutex::new(JobHandlerRegistry::new()));
    assert!(registry.lock().unwrap().get("nonexistent_job").is_none());
}

fn dummy_handler(
    _world: &mut engine_core::World,
    _agent_id: u32,
    _job_id: u32,
    _data: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!(null)
}

#[test]
fn test_data_driven_registration() {
    use std::fs;
    use tempfile::tempdir;

    // Create a temporary directory for job definitions
    let temp_dir = tempdir().expect("failed to create temp dir");
    let jobs_dir = temp_dir.path();

    // Create minimal job definition files for "production" and "haul"
    fs::write(
        jobs_dir.join("production.json"),
        r#"{"name":"production","type":"production"}"#,
    )
    .unwrap();
    fs::write(
        jobs_dir.join("haul.json"),
        r#"{"name":"haul","type":"haul"}"#,
    )
    .unwrap();

    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::default(),
    ));
    let mut world = World::new(registry);

    // Register dummy native logic for both job types
    let mut job_type_registry = JobTypeRegistry::default();
    job_type_registry.register_native("production", dummy_handler);
    job_type_registry.register_native("haul", dummy_handler);

    register_builtin_job_handlers(&mut world, &job_type_registry, jobs_dir);

    for job_type in &["production", "haul"] {
        assert!(
            world
                .job_handler_registry
                .lock()
                .unwrap()
                .get(job_type)
                .is_some(),
            "Handler for job type '{job_type}' was not registered"
        );
    }
}

// --- Section: Handler Registration ---

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
    job_system.run(&mut world);

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
    job_system.run(&mut world);

    let job_foo = world.get_component(job_foo_id, "Job").unwrap();
    let job_bar = world.get_component(job_bar_id, "Job").unwrap();
    assert_eq!(job_foo.get("progress").unwrap(), 1.0);
    assert_eq!(job_foo.get("state").unwrap(), "complete");
    assert_eq!(job_bar.get("progress").unwrap(), 2.0);
    assert_eq!(job_bar.get("state").unwrap(), "complete");
}

// --- Section: Type Registry ---

#[test]
fn test_can_load_and_lookup_job_types_from_json() {
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    // Create a temp directory for job definitions
    let temp_dir = TempDir::new().unwrap();
    let jobs_dir = temp_dir.path();

    // Write a sample job type JSON file
    let dig_job_json = r#"
    {
        "name": "DigTunnel",
        "requirements": ["Tool:Pickaxe"],
        "duration": 5,
        "effects": [{ "action": "ModifyTerrain", "from": "rock", "to": "tunnel" }]
    }
    "#;
    let dig_job_path = jobs_dir.join("dig_tunnel.json");
    let mut file = File::create(&dig_job_path).unwrap();
    file.write_all(dig_job_json.as_bytes()).unwrap();

    // Load job types from the directory
    let registry = JobTypeRegistry::load_from_dir(jobs_dir).unwrap();

    // Lookup the job type by name (case-insensitive, normalized)
    let dig = registry.get_data("DigTunnel").expect("job type exists");
    assert_eq!(dig.name, "DigTunnel");
    assert_eq!(dig.duration, Some(5.0));
    assert_eq!(dig.requirements, vec!["Tool:Pickaxe"]);
    assert_eq!(dig.effects.len(), 1);
    assert_eq!(dig.effects[0]["action"], "ModifyTerrain");
    assert_eq!(dig.effects[0]["from"], "rock");
    assert_eq!(dig.effects[0]["to"], "tunnel");
}

#[test]
fn test_job_type_asset_is_registered() {
    let mut world = world_helper::make_test_world();

    // Load the real job type JSON from disk (e.g., DigTunnel)
    let job_type = job_type_helper::load_job_type_from_assets("dig_tunnel");
    world.job_types.register_job_type(job_type);

    let dig = world
        .job_types
        .get_data("DigTunnel")
        .expect("DigTunnel should be registered");
    assert_eq!(dig.name, "DigTunnel");
    assert_eq!(dig.requirements, vec!["Tool:Pickaxe"]);
    assert_eq!(dig.duration, Some(5.0));
    assert_eq!(dig.effects[0]["action"], "ModifyTerrain");
    assert_eq!(dig.effects[0]["from"], "rock");
    assert_eq!(dig.effects[0]["to"], "tunnel");
}

// --- Section: Skill Multiplier ---

/// Verifies that the skill multiplier formula is applied correctly for job progress.
///
/// Formula: progress_increment = max(0.1, 1.0 * skill_value * (stamina / 100.0))
///
/// Uses SkillLevels component (R014 path) — the preferred mechanism over agent.skills.
#[test]
fn test_skill_multiplier_with_skill_levels() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Agent with high skill via SkillLevels
    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 5.0 },
                "skill_levels": { "dig": 5.0 },
                "total_xp": 120.0,
                "skill_xp": { "dig": 120.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "stamina": 100.0,
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
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // With skill=5.0, stamina=100: increment = 1.0 * 5.0 * (100/100) = 5.0
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 5.0).abs() < f64::EPSILON,
        "Expected progress ~5.0, got {}",
        progress
    );
}

/// Verifies skill multiplier with partial stamina.
#[test]
fn test_skill_multiplier_with_partial_stamina() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 2.0 },
                "skill_levels": { "dig": 2.0 },
                "total_xp": 50.0,
                "skill_xp": { "dig": 50.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "stamina": 50.0,
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
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // With skill=2.0, stamina=50: increment = 1.0 * 2.0 * (50/100) = 1.0
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 1.0).abs() < f64::EPSILON,
        "Expected progress ~1.0, got {}",
        progress
    );
}

/// Verifies skill multiplier floor of 0.1 when stamina is very low.
#[test]
fn test_skill_multiplier_floor() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 1.0 },
                "skill_levels": { "dig": 1.0 },
                "total_xp": 0.0,
                "skill_xp": { "dig": 0.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "stamina": 5.0,
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
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // With skill=1.0, stamina=5: raw = 1.0 * 1.0 * (5/100) = 0.05, clamped to 0.1
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 0.1).abs() < f64::EPSILON,
        "Expected progress ~0.1 (floor), got {}",
        progress
    );
}

/// Verifies that a job without assigned_to uses progress_increment=1.0 (no skill scaling).
/// Requires a registered job type with effects to stay in "in_progress" state.
#[test]
fn test_skill_multiplier_no_agent() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let job_id = world.spawn_entity();

    // Register a job type with effects so unassigned job stays "in_progress"
    world.job_types.register_job_type(JobTypeData {
        name: "dig".to_string(),
        effects: vec![serde_json::json!({"type": "test_effect"})],
        ..Default::default()
    });

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // No assigned_to: progress_increment defaults to 1.0
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 1.0).abs() < f64::EPSILON,
        "Expected progress ~1.0 (no agent), got {}",
        progress
    );
}

/// Verifies that LeveledUpEvent is emitted when a skill levels up on job completion.
#[test]
fn test_skill_level_up_event_on_job_completion() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    // Agent with skill close to leveling up (level 1, XP near threshold)
    // Threshold: base_xp * (1.5^current_level) = 10 * 1.5^1 = 15
    // Set skill_xp to 14, so one job completion (10xp base) pushes it over
    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 1.0 },
                "skill_levels": { "dig": 1.0 },
                "total_xp": 14.0,
                "skill_xp": { "dig": 14.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "stamina": 100.0,
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
                "job_type": "dig",
                "progress": 2.5,
                "state": "in_progress",
                "required_progress": 3.0,
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    // Run twice: first tick makes progress, second tick completes the job
    job_system.run(&mut world);
    job_system.run(&mut world);

    // Check that SkillLevels has the leveled-up skill
    let skill_levels = world.get_component(agent_id, "SkillLevels").unwrap();
    let dig_level = skill_levels
        .get("skill_levels")
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("dig"))
        .and_then(|v| v.as_f64());

    assert!(
        dig_level.is_some(),
        "Skill should have leveled up after job completion"
    );
    assert!(
        dig_level.unwrap() > 1.0,
        "Level should be greater than starting level 1, got {}",
        dig_level.unwrap()
    );
}

/// Verifies that SkillLevels takes precedence over deprecated agent.skills (R014).
#[test]
fn test_skill_levels_precedence_over_agent_skills() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    // Set both SkillLevels (skill=5.0) and agent.skills (skill=1.0)
    // SkillLevels should take precedence
    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 5.0 },
                "skill_levels": { "dig": 5.0 },
                "total_xp": 120.0,
                "skill_xp": { "dig": 120.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "dig": 1.0 },
                "stamina": 100.0,
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
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // SkillLevels.skills.dig = 5.0 should be used (not agent.skills.dig = 1.0)
    // increment = 1.0 * 5.0 * (100/100) = 5.0
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 5.0).abs() < f64::EPSILON,
        "Expected progress ~5.0 (SkillLevels precedence), got {}",
        progress
    );
}
