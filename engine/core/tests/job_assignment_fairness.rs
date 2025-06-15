#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::world::World;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use serde_json::json;

#[test]
fn test_job_assignment_fairness() {
    let mut world = world_helper::make_test_world();

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

    let result = job_board.claim_job(100, &mut world, 10);
    assert_eq!(
        result,
        JobAssignmentResult::Assigned(2),
        "Job 2 should be assigned for fairness"
    );
    let job2 = world.get_component(2, "Job").unwrap();
    assert_eq!(
        job2["assigned_to"], 100,
        "Job 2 should be assigned to agent 100"
    );
    assert_eq!(
        job2["assignment_count"], 2,
        "Job 2 assignment count should be incremented"
    );
    assert_eq!(
        job2["last_assigned_tick"], 10,
        "Job 2 last assigned tick should be updated"
    );
}

#[test]
fn test_job_assignment_dynamic_priority() {
    let mut world = world_helper::make_test_world();

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

    let result = job_board.claim_job(100, &mut world, 1);
    assert_eq!(
        result,
        JobAssignmentResult::Assigned(2),
        "Higher priority job (2) should be assigned first"
    );

    let mut job1 = world.get_component(1, "Job").unwrap().clone();
    job1["priority"] = json!(20);
    world.set_component(1, "Job", job1).unwrap();

    let mut job2 = world.get_component(2, "Job").unwrap().clone();
    job2.as_object_mut().unwrap().remove("assigned_to");
    job2["status"] = json!("pending");
    world.set_component(2, "Job", job2).unwrap();

    let mut agent = world.get_component(100, "Agent").unwrap().clone();
    agent.as_object_mut().unwrap().remove("current_job");
    agent["state"] = json!("idle");
    world.set_component(100, "Agent", agent).unwrap();

    job_board.update(&world);

    let result = job_board.claim_job(100, &mut world, 2);
    assert_eq!(
        result,
        JobAssignmentResult::Assigned(1),
        "Now job 1 should be assigned due to higher priority"
    );
    let job1 = world.get_component(1, "Job").unwrap();
    assert_eq!(
        job1["assigned_to"], 100,
        "Job 1 should be assigned to agent 100"
    );
    assert_eq!(
        job1["assignment_count"], 1,
        "Job 1 assignment count should be incremented"
    );
    assert_eq!(
        job1["last_assigned_tick"], 2,
        "Job 1 last assigned tick should be updated"
    );
}

#[test]
fn test_job_assignment_persistence() {
    let mut world = world_helper::make_test_world();

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

    let tmp = tempfile::NamedTempFile::new().unwrap();
    world.save_to_file(tmp.path()).unwrap();

    let loaded = World::load_from_file(tmp.path(), world.registry.clone()).unwrap();
    let job = loaded.get_component(1, "Job").unwrap();
    assert_eq!(
        job["assignment_count"], 2,
        "Assignment count should persist after save/load"
    );
    assert_eq!(
        job["last_assigned_tick"], 42,
        "Last assigned tick should persist after save/load"
    );
}
