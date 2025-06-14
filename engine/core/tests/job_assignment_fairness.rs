use engine_core::ecs::world::World;
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
    Arc::new(Mutex::new(registry))
}

#[test]
fn test_job_assignment_fairness() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Add two jobs with the same priority, but one has been assigned more often
    world
        .set_component(
            1,
            "Job",
            json!({
                "id": 1,
                "job_type": "dig",
                "status": "pending",
                "priority": 10,
                "assignment_count": 3,
                "last_assigned_tick": 5,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(1);

    world
        .set_component(
            2,
            "Job",
            json!({
                "id": 2,
                "job_type": "dig",
                "status": "pending",
                "priority": 10,
                "assignment_count": 1,
                "last_assigned_tick": 2,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(2);

    // Add an agent
    world
        .set_component(
            100,
            "Agent",
            json!({
                "entity_id": 100,
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(100);

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    // The agent should get job 2 (least assigned)
    let result = job_board.claim_job(100, &mut world, 10);
    assert_eq!(
        result,
        engine_core::systems::job_board::JobAssignmentResult::Assigned(2)
    );
    let job2 = world.get_component(2, "Job").unwrap();
    assert_eq!(job2["assigned_to"], 100);
    assert_eq!(job2["assignment_count"], 2);
    assert_eq!(job2["last_assigned_tick"], 10);
}

#[test]
fn test_job_assignment_dynamic_priority() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Two jobs, one with lower priority initially
    world
        .set_component(
            1,
            "Job",
            json!({
                "id": 1,
                "job_type": "dig",
                "status": "pending",
                "priority": 5,
                "assignment_count": 0,
                "last_assigned_tick": 0,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(1);

    world
        .set_component(
            2,
            "Job",
            json!({
                "id": 2,
                "job_type": "dig",
                "status": "pending",
                "priority": 10,
                "assignment_count": 0,
                "last_assigned_tick": 0,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(2);

    // Agent
    world
        .set_component(
            100,
            "Agent",
            json!({
                "entity_id": 100,
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(100);

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    // Agent should get job 2 (higher priority)
    let result = job_board.claim_job(100, &mut world, 1);
    assert_eq!(
        result,
        engine_core::systems::job_board::JobAssignmentResult::Assigned(2)
    );

    // Now, increase job 1's priority and make it available again
    let mut job1 = world.get_component(1, "Job").unwrap().clone();
    job1["priority"] = json!(20);
    world.set_component(1, "Job", job1).unwrap();

    // Unassign job 2 for the next test
    let mut job2 = world.get_component(2, "Job").unwrap().clone();
    job2.as_object_mut().unwrap().remove("assigned_to");
    job2["status"] = json!("pending");
    world.set_component(2, "Job", job2).unwrap();

    // Agent is idle again
    let mut agent = world.get_component(100, "Agent").unwrap().clone();
    agent.as_object_mut().unwrap().remove("current_job");
    agent["state"] = json!("idle");
    world.set_component(100, "Agent", agent).unwrap();

    job_board.update(&world);

    // Agent should now get job 1 (now highest priority)
    let result = job_board.claim_job(100, &mut world, 2);
    assert_eq!(
        result,
        engine_core::systems::job_board::JobAssignmentResult::Assigned(1)
    );
    let job1 = world.get_component(1, "Job").unwrap();
    assert_eq!(job1["assigned_to"], 100);
    assert_eq!(job1["assignment_count"], 1);
    assert_eq!(job1["last_assigned_tick"], 2);
}

#[test]
fn test_job_assignment_persistence() {
    let registry = setup_registry();
    let mut world = World::new(registry.clone());

    // Job with fairness metadata
    world
        .set_component(
            1,
            "Job",
            json!({
                "id": 1,
                "job_type": "dig",
                "status": "pending",
                "priority": 5,
                "assignment_count": 2,
                "last_assigned_tick": 42,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(1);

    // Save to a temp file
    let tmp = tempfile::NamedTempFile::new().unwrap();
    world.save_to_file(tmp.path()).unwrap();

    // Load world
    let loaded = World::load_from_file(tmp.path(), registry).unwrap();
    let job = loaded.get_component(1, "Job").unwrap();
    assert_eq!(job["assignment_count"], 2);
    assert_eq!(job["last_assigned_tick"], 42);
}
