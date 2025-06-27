#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use serde_json::json;

#[test]
fn test_job_board_tracks_unassigned_jobs() {
    let mut world = world_helper::make_test_world();
    let job1 = json!({"job_type": "mine", "state": "pending", "category": "mining"});
    let job2 =
        json!({"job_type": "haul", "state": "pending", "assigned_to": 42, "category": "hauling"});
    let eid1 = world.spawn_entity();
    let eid2 = world.spawn_entity();
    world.set_component(eid1, "Job", job1.clone()).unwrap();
    world.set_component(eid2, "Job", job2.clone()).unwrap();

    let mut board = JobBoard::default();
    board.update(&world);

    assert!(board.jobs.contains(&eid1));
    assert!(!board.jobs.contains(&eid2));
}

#[test]
fn test_job_assignment_claims_job() {
    let mut world = world_helper::make_test_world();
    let job = json!({"job_type": "build", "state": "pending", "category": "construction"});
    let eid = world.spawn_entity();
    let actor_eid = world.spawn_entity();
    world.set_component(eid, "Job", job.clone()).unwrap();

    let mut board = JobBoard::default();
    board.update(&world);

    let result = board.claim_job(actor_eid, &mut world, 0);
    assert_eq!(result, JobAssignmentResult::Assigned(eid));

    let assigned_job = world.get_component(eid, "Job").unwrap();
    assert_eq!(
        assigned_job.get("assigned_to").and_then(|v| v.as_u64()),
        Some(actor_eid as u64)
    );
}

#[test]
fn test_job_assignment_no_jobs_available() {
    let mut world = world_helper::make_test_world();
    let actor_eid = world.spawn_entity();

    let mut board = JobBoard::default();
    board.update(&world);

    let result = board.claim_job(actor_eid, &mut world, 0);
    assert_eq!(result, JobAssignmentResult::NoJobsAvailable);
}
