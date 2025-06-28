#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, assign_jobs};
use serde_json::json;

/// Test: Conditional child job is spawned when parent job fails.
#[test]
fn test_conditional_child_spawn_on_failure() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Create a dependency job that has failed
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

    // Parent job depends on the failed job, so it will fail
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
                "dependencies": [dep_id.to_string()],  // <-- FIX: use string, not integer
                "conditional_children": [
                    {
                        "spawn_if": { "field": "state", "equals": "failed" },
                        "job": {
                            "job_type": "repair",
                            "state": "pending",
                            "priority": 1,
                            "category": "test"
                        }
                    }
                ]
            }),
        )
        .unwrap();

    // Agent to work on the job
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

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    // Run job system to trigger dependency failure and conditional spawn
    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    // Find the spawned child job
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
    assert_eq!(child["state"], "pending");
}

/// Test: Conditional child job is spawned when world state matches.
#[test]
fn test_conditional_child_spawn_on_world_state() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    world.set_global_resource_amount("food", 5.0);

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
                "conditional_children": [
                    {
                        "spawn_if": { "world_state": { "resource": "food", "lte": 10.0 } },
                        "job": {
                            "job_type": "gather_food",
                            "state": "pending",
                            "priority": 1,
                            "category": "test"
                        }
                    }
                ]
            }),
        )
        .unwrap();

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

    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let mut job_system = JobSystem::new();
    for _ in 0..5 {
        job_system.run(&mut world, None);
    }

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
        child["state"] == "pending" || child["state"] == "in_progress",
        "Child job state should be 'pending' or 'in_progress', got {:?}",
        child["state"]
    );
}
