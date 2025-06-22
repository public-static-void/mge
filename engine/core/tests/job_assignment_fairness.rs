#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::world::World;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use serde_json::json;

#[test]
fn test_job_assignment_fairness() {
    let mut world = world_helper::make_test_world();

    let job1 = world.spawn_entity();
    world
        .set_component(
            job1,
            "Job",
            json!({
                "id": job1,
                "job_type": "dig",
                "status": "pending",
                "priority": 10,
                "assignment_count": 3,
                "last_assigned_tick": 5,
                "category": "mining"
            }),
        )
        .unwrap();

    let job2 = world.spawn_entity();
    world
        .set_component(
            job2,
            "Job",
            json!({
                "id": job2,
                "job_type": "dig",
                "status": "pending",
                "priority": 10,
                "assignment_count": 1,
                "last_assigned_tick": 2,
                "category": "mining"
            }),
        )
        .unwrap();

    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "state": "idle"
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    let result = job_board.claim_job(agent, &mut world, 10);
    assert_eq!(
        result,
        JobAssignmentResult::Assigned(job2),
        "Job 2 should be assigned for fairness"
    );
    let job2_obj = world.get_component(job2, "Job").unwrap();
    assert_eq!(
        job2_obj["assigned_to"], agent,
        "Job 2 should be assigned to agent"
    );
    assert_eq!(
        job2_obj["assignment_count"], 2,
        "Job 2 assignment count should be incremented"
    );
    assert_eq!(
        job2_obj["last_assigned_tick"], 10,
        "Job 2 last assigned tick should be updated"
    );
}

#[test]
fn test_job_assignment_dynamic_priority() {
    let mut world = world_helper::make_test_world();

    let job1 = world.spawn_entity();
    world
        .set_component(
            job1,
            "Job",
            json!({
                "id": job1,
                "job_type": "dig",
                "status": "pending",
                "priority": 5,
                "assignment_count": 0,
                "last_assigned_tick": 0,
                "category": "mining"
            }),
        )
        .unwrap();

    let job2 = world.spawn_entity();
    world
        .set_component(
            job2,
            "Job",
            json!({
                "id": job2,
                "job_type": "dig",
                "status": "pending",
                "priority": 10,
                "assignment_count": 0,
                "last_assigned_tick": 0,
                "category": "mining"
            }),
        )
        .unwrap();

    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "state": "idle"
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world);

    let result = job_board.claim_job(agent, &mut world, 1);
    assert_eq!(
        result,
        JobAssignmentResult::Assigned(job2),
        "Higher priority job (2) should be assigned first"
    );

    let mut job1_obj = world.get_component(job1, "Job").unwrap().clone();
    job1_obj["priority"] = json!(20);
    world.set_component(job1, "Job", job1_obj).unwrap();

    let mut job2_obj = world.get_component(job2, "Job").unwrap().clone();
    job2_obj.as_object_mut().unwrap().remove("assigned_to");
    job2_obj["status"] = json!("pending");
    world.set_component(job2, "Job", job2_obj).unwrap();

    let mut agent_obj = world.get_component(agent, "Agent").unwrap().clone();
    agent_obj.as_object_mut().unwrap().remove("current_job");
    agent_obj["state"] = json!("idle");
    world.set_component(agent, "Agent", agent_obj).unwrap();

    job_board.update(&world);

    let result = job_board.claim_job(agent, &mut world, 2);
    assert_eq!(
        result,
        JobAssignmentResult::Assigned(job1),
        "Now job 1 should be assigned due to higher priority"
    );
    let job1_obj = world.get_component(job1, "Job").unwrap();
    assert_eq!(
        job1_obj["assigned_to"], agent,
        "Job 1 should be assigned to agent"
    );
    assert_eq!(
        job1_obj["assignment_count"], 1,
        "Job 1 assignment count should be incremented"
    );
    assert_eq!(
        job1_obj["last_assigned_tick"], 2,
        "Job 1 last assigned tick should be updated"
    );
}

#[test]
fn test_job_assignment_persistence() {
    let mut world = world_helper::make_test_world();

    let job1 = world.spawn_entity();
    world
        .set_component(
            job1,
            "Job",
            json!({
                "id": job1,
                "job_type": "dig",
                "status": "pending",
                "priority": 5,
                "assignment_count": 2,
                "last_assigned_tick": 42,
                "category": "mining"
            }),
        )
        .unwrap();

    let tmp = tempfile::NamedTempFile::new().unwrap();
    world.save_to_file(tmp.path()).unwrap();

    let loaded = World::load_from_file(tmp.path(), world.registry.clone()).unwrap();
    let job = loaded.get_component(job1, "Job").unwrap();
    assert_eq!(
        job["assignment_count"], 2,
        "Assignment count should persist after save/load"
    );
    assert_eq!(
        job["last_assigned_tick"], 42,
        "Last assigned tick should persist after save/load"
    );
}
