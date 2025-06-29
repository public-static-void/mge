#[path = "helpers/world.rs"]
mod world_helper;

use serde_json::json;

#[test]
fn test_fifo_job_assignment_policy() {
    use engine_core::systems::job::board::job_board::{FifoPolicy, JobBoard};
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Agent: idle
    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "skills": { "dig": 1.0 },
                "state": "idle"
            }),
        )
        .unwrap();

    // Job 1: dig, priority 10 (added first)
    let job1 = world.spawn_entity();
    world
        .set_component(
            job1,
            "Job",
            json!({
                "job_type": "dig",
                "state": "pending",
                "priority": 10,
                "category": "mining",
                "created_at": 1
            }),
        )
        .unwrap();

    // Job 2: dig, priority 1 (added second)
    let job2 = world.spawn_entity();
    world
        .set_component(
            job2,
            "Job",
            json!({
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "created_at": 2
            }),
        )
        .unwrap();

    // Use FIFO policy
    let mut job_board = JobBoard::with_policy(Box::new(FifoPolicy));
    job_board.update(&world);

    engine_core::systems::job::ai::logic::assign_jobs(&mut world, &mut job_board);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    // Should get job1 (the first-added), not job2 (the highest priority)
    assert_eq!(agent_obj["current_job"], job1);
}

#[test]
fn test_lifo_job_assignment_policy() {
    use engine_core::systems::job::board::job_board::{JobBoard, LifoPolicy};
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Agent: idle
    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            serde_json::json!({
                "entity_id": agent,
                "skills": { "dig": 1.0 },
                "state": "idle"
            }),
        )
        .unwrap();

    // Job 1: dig, priority 10 (added first)
    let job1 = world.spawn_entity();
    world
        .set_component(
            job1,
            "Job",
            serde_json::json!({
                "job_type": "dig",
                "state": "pending",
                "priority": 10,
                "category": "mining",
                "created_at": 1
            }),
        )
        .unwrap();

    // Job 2: dig, priority 1 (added second)
    let job2 = world.spawn_entity();
    world
        .set_component(
            job2,
            "Job",
            serde_json::json!({
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "created_at": 2
            }),
        )
        .unwrap();

    // Use LIFO policy
    let mut job_board = JobBoard::with_policy(Box::new(LifoPolicy));
    job_board.update(&world);

    engine_core::systems::job::ai::logic::assign_jobs(&mut world, &mut job_board);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    // Should get job2 (the last-added), not job1 (the highest priority)
    assert_eq!(agent_obj["current_job"], job2);
}
