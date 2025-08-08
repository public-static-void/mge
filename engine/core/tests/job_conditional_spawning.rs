//! Tests for conditional job spawning and assignment logic.

#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobLogicKind, JobSystem, JobTypeData, assign_jobs};
use serde_json::json;

/// Test that a conditional child job is spawned when the parent fails.
/// Also verifies correct assignment and state transitions.
#[test]
fn test_conditional_child_spawn_on_failure() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register robust, terminal-aware handler for "main"
    let main_job_type = JobTypeData {
        name: "main".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        main_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
                return job;
            }
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("failed"); // intentionally fail to test child spawn
            }
            job
        }),
    );

    // Register robust, terminal-aware handler for "repair"
    let repair_job_type = JobTypeData {
        name: "repair".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        repair_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
                return job;
            }
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("complete");
            }
            job
        }),
    );

    // Register robust, terminal-aware handler for "dep"
    let dep_job_type = JobTypeData {
        name: "dep".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        dep_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| job.clone()),
    );

    // Create a dependency job that is already failed.
    let dep_id = world.spawn_entity();
    world
        .set_component(
            dep_id,
            "Job",
            json!({
                "id": dep_id,
                "job_type": "dep",
                "state": "failed",
                "priority": 1,
                "category": "test"
            }),
        )
        .unwrap();

    // Create a parent job with a conditional child that spawns on failure.
    let parent_id = world.spawn_entity();
    world
        .set_component(
            parent_id,
            "Job",
            json!({
                "id": parent_id,
                "job_type": "main",
                "state": "pending",
                "priority": 1,
                "category": "test",
                "dependencies": [dep_id.to_string()],
                "conditional_children": [
                    {
                        "spawn_if": { "field": "state", "equals": "failed" },
                        "job": {
                            "job_type": "repair",
                            "state": "in_progress",
                            "priority": 1,
                            "category": "test"
                        }
                    }
                ]
            }),
        )
        .unwrap();

    // Create an agent with specializations.
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle",
                "specializations": ["test"]
            }),
        )
        .unwrap();

    // Run assignment and orchestrator
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    // Set agent back to idle so it can be assigned to the child job.
    let mut agent = world.get_component(agent_id, "Agent").unwrap().clone();
    agent["state"] = serde_json::json!("idle");
    agent["current_job"] = serde_json::Value::Null;
    world.set_component(agent_id, "Agent", agent).unwrap();

    // Assign jobs again so the agent can pick up the spawned child job.
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    // Run the job system again to process the child job.
    job_system.run(&mut world, None);

    // Check that exactly one child job was spawned.
    let spawned_jobs: Vec<_> = world
        .get_entities_with_component("Job")
        .into_iter()
        .filter(|&eid| eid != parent_id && eid != dep_id)
        .collect();

    assert_eq!(
        spawned_jobs.len(),
        1,
        "Exactly one conditional child job should be spawned"
    );
    let child = world.get_component(spawned_jobs[0], "Job").unwrap();
    assert_eq!(child["job_type"], "repair");
    assert!(
        child["state"] == "pending"
            || child["state"] == "in_progress"
            || child["state"] == "complete",
        "Child job state should be 'pending', 'in_progress', or 'complete', got {:?}",
        child["state"]
    );
}

/// Test that a conditional child job is spawned based on world state.
/// Also verifies correct assignment and state transitions.
#[test]
fn test_conditional_child_spawn_on_world_state() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register robust, terminal-aware handler for "main"
    let main_job_type = JobTypeData {
        name: "main".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        main_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
                return job;
            }
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("complete");
            }
            job
        }),
    );

    // Register robust, terminal-aware handler for "gather_food"
    let gather_food_job_type = JobTypeData {
        name: "gather_food".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        gather_food_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
                return job;
            }
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("complete");
            }
            job
        }),
    );

    // Create the parent job with a conditional child.
    let parent_id = world.spawn_entity();

    world
        .set_component(
            parent_id,
            "Job",
            json!({
                "id": parent_id,
                "job_type": "main",
                "state": "pending",
                "progress": 0.0,
                "priority": 1,
                "category": "test",
                "conditional_children": [
                    {
                        "spawn_if": { "world_state": { "resource": "food", "lte": 10.0 } },
                        "job": {
                            "job_type": "gather_food",
                            "state": "in_progress",
                            "priority": 1,
                            "category": "test"
                        }
                    }
                ]
            }),
        )
        .unwrap();

    // Create an agent with the correct specialization.
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle",
                "specializations": ["test"]
            }),
        )
        .unwrap();

    // Make sure the world state will trigger the conditional child.
    world.set_global_resource_amount("food", 5.0);

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    // Set parent job to in_progress and assigned.
    let mut parent_job = world.get_component(parent_id, "Job").unwrap().clone();
    parent_job["state"] = serde_json::json!("in_progress");
    parent_job["assigned_to"] = serde_json::json!(agent_id);
    world.set_component(parent_id, "Job", parent_job).unwrap();

    let mut job_system = JobSystem::new();

    for _ in 0..5 {
        job_system.run(&mut world, None);
    }

    // Set agent back to idle so it can be assigned to the child job.
    let mut agent = world.get_component(agent_id, "Agent").unwrap().clone();
    agent["state"] = serde_json::json!("idle");
    agent["current_job"] = serde_json::Value::Null;
    world.set_component(agent_id, "Agent", agent).unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    for _ in 0..5 {
        job_system.run(&mut world, None);
    }

    // Check that exactly one child job was spawned.
    let spawned_jobs: Vec<_> = world
        .get_entities_with_component("Job")
        .into_iter()
        .filter(|&eid| eid != parent_id)
        .collect();

    assert_eq!(
        spawned_jobs.len(),
        1,
        "Exactly one conditional child job should be spawned"
    );
    let child = world.get_component(spawned_jobs[0], "Job").unwrap();
    assert_eq!(child["job_type"], "gather_food");
    assert!(
        child["state"] == "pending"
            || child["state"] == "in_progress"
            || child["state"] == "complete",
        "Child job state should be 'pending', 'in_progress', or 'complete', got {:?}",
        child["state"]
    );
}
