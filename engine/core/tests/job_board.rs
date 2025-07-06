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

#[test]
fn test_job_board_metadata_and_policy_access() {
    use engine_core::systems::job::board::job_board::JobBoard;
    use serde_json::json;

    let mut world = world_helper::make_test_world();

    // Add jobs with different priorities
    let job1 = json!({"priority": 5, "state": "pending", "job_type": "test", "category": "test"});
    let job2 = json!({"priority": 10, "state": "pending", "job_type": "test", "category": "test"});
    let job3 = json!({"priority": 1, "state": "pending", "job_type": "test", "category": "test"});
    let eid1 = world.spawn_entity();
    let eid2 = world.spawn_entity();
    let eid3 = world.spawn_entity();
    world.set_component(eid1, "Job", job1).unwrap();
    world.set_component(eid2, "Job", job2).unwrap();
    world.set_component(eid3, "Job", job3).unwrap();

    // Default: PriorityPolicy
    let mut board = JobBoard::default();
    board.update(&world);
    let jobs = board.jobs_with_metadata(&world);
    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].eid, eid2); // Highest priority first
    assert_eq!(jobs[1].eid, eid1);
    assert_eq!(jobs[2].eid, eid3);

    // Change to FIFO
    board.set_policy("fifo").unwrap();
    board.update(&world);
    let jobs = board.jobs_with_metadata(&world);
    assert_eq!(jobs[0].eid, eid1); // Oldest first (eid1)
    assert_eq!(jobs[1].eid, eid2);
    assert_eq!(jobs[2].eid, eid3);

    // Change to LIFO
    board.set_policy("lifo").unwrap();
    board.update(&world);
    let jobs = board.jobs_with_metadata(&world);
    assert_eq!(jobs[0].eid, eid3); // Newest first (eid3)
    assert_eq!(jobs[1].eid, eid2);
    assert_eq!(jobs[2].eid, eid1);

    // Test get/set priority
    assert_eq!(board.get_priority(&world, eid1), Some(5));
    board.set_priority(&mut world, eid1, 42).unwrap();
    assert_eq!(board.get_priority(&world, eid1), Some(42));
}
