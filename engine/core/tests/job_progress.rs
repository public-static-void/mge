use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::system::JobSystem;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn setup_world() -> World {
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
    let registry = Arc::new(Mutex::new(registry));
    World::new(registry)
}

#[test]
fn test_job_progress_depends_on_agent_skill() {
    let mut world = setup_world();

    // Agent with high skill
    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "skills": { "dig": 3.0 },
                "stamina": 100.0,
                "state": "working",
                "current_job": 10
            }),
        )
        .unwrap();
    world.entities.push(1);

    // Job assigned to agent
    world
        .set_component(
            10,
            "Job",
            json!({
                "id": 10,
                "job_type": "dig",
                "progress": 0.0,
                "status": "in_progress",
                "assigned_to": 1
            }),
        )
        .unwrap();
    world.entities.push(10);

    let mut job_system = JobSystem::new();

    // Run system (simulate one tick)
    job_system.run(&mut world, None);

    let job = world.get_component(10, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap();
    assert!(
        progress > 1.0,
        "Progress should be > 1.0 for high skill agent"
    );
}

#[test]
fn test_job_progress_slow_with_low_stamina() {
    let mut world = setup_world();

    // Agent with skill but low stamina
    world
        .set_component(
            2,
            "Agent",
            json!({
                "entity_id": 2,
                "skills": { "dig": 2.0 },
                "stamina": 10.0,
                "state": "working",
                "current_job": 20
            }),
        )
        .unwrap();
    world.entities.push(2);

    // Job assigned to agent
    world
        .set_component(
            20,
            "Job",
            json!({
                "id": 20,
                "job_type": "dig",
                "progress": 0.0,
                "status": "in_progress",
                "assigned_to": 2
            }),
        )
        .unwrap();
    world.entities.push(20);

    let mut job_system = JobSystem::new();

    // Run system (simulate one tick)
    job_system.run(&mut world, None);

    let job = world.get_component(20, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap();
    assert!(progress < 2.0, "Progress should be slower with low stamina");
}

#[test]
fn test_job_completes_when_progress_threshold_met() {
    let mut world = setup_world();

    // Agent with moderate skill
    world
        .set_component(
            3,
            "Agent",
            json!({
                "entity_id": 3,
                "skills": { "dig": 1.0 },
                "stamina": 100.0,
                "state": "working",
                "current_job": 30
            }),
        )
        .unwrap();
    world.entities.push(3);

    // Job nearly complete
    world
        .set_component(
            30,
            "Job",
            json!({
                "id": 30,
                "job_type": "dig",
                "progress": 2.5,
                "status": "in_progress",
                "assigned_to": 3
            }),
        )
        .unwrap();
    world.entities.push(30);

    let mut job_system = JobSystem::new();

    // Run system (simulate one tick)
    job_system.run(&mut world, None);

    let job = world.get_component(30, "Job").unwrap();
    let status = job.get("status").and_then(|v| v.as_str()).unwrap();
    assert_eq!(
        status, "complete",
        "Job should complete when progress >= 3.0"
    );
}
