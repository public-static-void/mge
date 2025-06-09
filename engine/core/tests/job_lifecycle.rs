use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::{JobSystem, assign_jobs};
use engine_core::systems::job_board::JobBoard;
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Helper to register all schemas needed for these tests
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
    Arc::new(Mutex::new(registry))
}

#[test]
fn test_agent_state_and_job_completion() {
    let registry = setup_registry();
    let mut world = World::new(registry);

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
                "priority": 1
            }),
        )
        .unwrap();
    world.entities.push(100);

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    let job = world.get_component(100, "Job").unwrap();

    assert_eq!(agent["current_job"], 100);
    assert_eq!(agent["state"], "working");
    assert_eq!(job["assigned_to"], 1);

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
    assert!(agent.get("current_job").is_none());
    assert_eq!(agent["state"], "idle");
}

#[test]
fn test_job_preemption_and_reassignment() {
    let registry = setup_registry();
    let mut world = World::new(registry);

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
                "priority": 1
            }),
        )
        .unwrap();
    world.entities.push(100);

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    let job = world.get_component(100, "Job").unwrap();
    assert_eq!(agent["current_job"], 100);
    assert_eq!(agent["state"], "working");
    assert_eq!(job["assigned_to"], 1);

    // Now, a higher-priority job appears
    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "dig",
                "status": "pending",
                "priority": 10
            }),
        )
        .unwrap();
    world.entities.push(200);

    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    let job100 = world.get_component(100, "Job").unwrap();
    let job200 = world.get_component(200, "Job").unwrap();

    // The agent will still be working on job 100 (no preemption in current logic)
    assert_eq!(agent["current_job"], 100);
    assert_eq!(agent["state"], "working");
    assert_eq!(job100["assigned_to"], 1);
    assert!(job200.get("assigned_to").is_none());
}
