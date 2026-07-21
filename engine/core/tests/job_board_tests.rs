#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::ai::assign_jobs;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use serde_json::json;

// --- Section: Board ---

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
    board.update(&world, 0, &[]);

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
    board.update(&world, 0, &[]);

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
    board.update(&world, 0, &[]);

    let result = board.claim_job(actor_eid, &mut world, 0);
    assert_eq!(result, JobAssignmentResult::NoJobsAvailable);
}

#[test]
fn test_job_board_metadata_and_policy_access() {
    use engine_core::systems::job::board::job_board::JobBoard;

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
    board.update(&world, 0, &[]);
    let jobs = board.jobs_with_metadata(&world);
    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].eid, eid2); // Highest priority first
    assert_eq!(jobs[1].eid, eid1);
    assert_eq!(jobs[2].eid, eid3);

    // Change to FIFO
    board.set_policy("fifo").unwrap();
    board.update(&world, 0, &[]);
    let jobs = board.jobs_with_metadata(&world);
    assert_eq!(jobs[0].eid, eid1); // Oldest first (eid1)
    assert_eq!(jobs[1].eid, eid2);
    assert_eq!(jobs[2].eid, eid3);

    // Change to LIFO
    board.set_policy("lifo").unwrap();
    board.update(&world, 0, &[]);
    let jobs = board.jobs_with_metadata(&world);
    assert_eq!(jobs[0].eid, eid3); // Newest first (eid3)
    assert_eq!(jobs[1].eid, eid2);
    assert_eq!(jobs[2].eid, eid1);

    // Test get/set priority
    assert_eq!(board.get_priority(&world, eid1), Some(5));
    board.set_priority(&mut world, eid1, 42).unwrap();
    assert_eq!(board.get_priority(&world, eid1), Some(42));
}

// --- Section: Assignment Fairness ---

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
                "state": "pending",
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
                "state": "pending",
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
    job_board.update(&world, 10, &[]);

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
                "state": "pending",
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
                "state": "pending",
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
    job_board.update(&world, 1, &[]);

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
    job2_obj["state"] = json!("pending");
    world.set_component(job2, "Job", job2_obj).unwrap();

    let mut agent_obj = world.get_component(agent, "Agent").unwrap().clone();
    agent_obj.as_object_mut().unwrap().remove("current_job");
    agent_obj["state"] = json!("idle");
    world.set_component(agent, "Agent", agent_obj).unwrap();

    job_board.update(&world, 2, &[]);

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
                "state": "pending",
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

// --- Section: Category Assignment ---

#[test]
fn test_agent_prefers_job_matching_specialization_category() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Agent 1 specializes in hauling
    let agent1_eid = world.spawn_entity();
    world
        .set_component(
            agent1_eid,
            "Agent",
            json!({
                "entity_id": agent1_eid,
                "specializations": ["hauling"],
                "skills": {},
                "preferences": {},
                "state": "idle"
            }),
        )
        .unwrap();

    // Agent 2 specializes in construction
    let agent2_eid = world.spawn_entity();
    world
        .set_component(
            agent2_eid,
            "Agent",
            json!({
                "entity_id": agent2_eid,
                "specializations": ["construction"],
                "skills": {},
                "preferences": {},
                "state": "idle"
            }),
        )
        .unwrap();

    // Job 1: hauling
    let job1_eid = world.spawn_entity();
    world
        .set_component(
            job1_eid,
            "Job",
            json!({
                "id": job1_eid,
                "job_type": "move_items",
                "category": "hauling",
                "state": "pending"
            }),
        )
        .unwrap();

    // Job 2: construction
    let job2_eid = world.spawn_entity();
    world
        .set_component(
            job2_eid,
            "Job",
            json!({
                "id": job2_eid,
                "job_type": "build_wall",
                "category": "construction",
                "state": "pending"
            }),
        )
        .unwrap();

    // Job 3: crafting (no agent specializes in this)
    let job3_eid = world.spawn_entity();
    world
        .set_component(
            job3_eid,
            "Job",
            json!({
                "id": job3_eid,
                "job_type": "make_tools",
                "category": "crafting",
                "state": "pending"
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    // Agent 1 should get hauling job
    let agent1 = world.get_component(agent1_eid, "Agent").unwrap();
    let assigned_job1 = agent1.get("current_job").and_then(|v| v.as_u64()).unwrap() as u32;
    assert_eq!(
        assigned_job1, job1_eid,
        "Agent 1 should be assigned the hauling job"
    );

    // Agent 2 should get construction job
    let agent2 = world.get_component(agent2_eid, "Agent").unwrap();
    let assigned_job2 = agent2.get("current_job").and_then(|v| v.as_u64()).unwrap() as u32;
    assert_eq!(
        assigned_job2, job2_eid,
        "Agent 2 should be assigned the construction job"
    );

    // Crafting job should remain unassigned
    let job3 = world.get_component(job3_eid, "Job").unwrap();
    assert!(
        job3.get("assigned_to").is_none_or(|v| v.is_null()),
        "Crafting job should remain unassigned"
    );
}

// --- Section: Priority Aging ---

#[test]
fn test_high_priority_job_is_assigned_first() {
    let mut world = world_helper::make_test_world();

    let high_eid = world.spawn_entity();
    let low_eid = world.spawn_entity();

    world
        .set_component(
            high_eid,
            "Job",
            json!({
                "id": high_eid,
                "job_type": "urgent",
                "state": "pending",
                "priority": 100,
                "creation_tick": 0,
                "category": "priority"
            }),
        )
        .unwrap();

    world
        .set_component(
            low_eid,
            "Job",
            json!({
                "id": low_eid,
                "job_type": "background",
                "state": "pending",
                "priority": 1,
                "creation_tick": 0,
                "category": "background"
            }),
        )
        .unwrap();

    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();

    // No shortages in this test; pass empty vector
    let shortage_kinds = vec![];

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &shortage_kinds);
    let result = job_board.claim_job(agent_eid, &mut world, 0);
    assert_eq!(result, JobAssignmentResult::Assigned(high_eid));
}

#[test]
fn test_low_priority_job_is_assigned_after_aging() {
    let mut world = world_helper::make_test_world();

    let high_eid = world.spawn_entity();
    let low_eid = world.spawn_entity();

    world
        .set_component(
            high_eid,
            "Job",
            json!({
                "id": high_eid,
                "job_type": "urgent",
                "state": "pending",
                "priority": 100,
                "creation_tick": 0,
                "category": "priority"
            }),
        )
        .unwrap();

    world
        .set_component(
            low_eid,
            "Job",
            json!({
                "id": low_eid,
                "job_type": "background",
                "state": "pending",
                "priority": 1,
                "creation_tick": 0,
                "category": "background"
            }),
        )
        .unwrap();

    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();

    let shortage_kinds = vec![];

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &shortage_kinds);
    let result = job_board.claim_job(agent_eid, &mut world, 0);
    assert_eq!(result, JobAssignmentResult::Assigned(high_eid));
    // Mark the high-priority job complete
    let mut job = world.get_component(high_eid, "Job").unwrap().clone();
    job["state"] = json!("complete");
    world.set_component(high_eid, "Job", job).unwrap();
    // Set agent to idle again
    let mut agent = world.get_component(agent_eid, "Agent").unwrap().clone();
    agent["state"] = json!("idle");
    world.set_component(agent_eid, "Agent", agent).unwrap();

    // After sufficient ticks, low-priority job should be assigned due to aging
    let mut assigned = false;
    for tick in 1..=200 {
        job_board.update(&world, tick, &shortage_kinds);
        let result = job_board.claim_job(agent_eid, &mut world, tick);
        if result == JobAssignmentResult::Assigned(low_eid) {
            assigned = true;
            break;
        }
        let mut agent = world.get_component(agent_eid, "Agent").unwrap().clone();
        agent["state"] = json!("idle");
        world.set_component(agent_eid, "Agent", agent).unwrap();
    }
    assert!(assigned, "Low-priority job was not assigned after aging");
}

#[test]
fn test_job_priority_can_be_bumped_by_world_event() {
    let mut world = world_helper::make_test_world();

    let job_eid = world.spawn_entity();
    world
        .set_component(
            job_eid,
            "Job",
            json!({
                "id": job_eid,
                "job_type": "critical",
                "state": "pending",
                "priority": 10,
                "creation_tick": 0,
                "category": "critical"
            }),
        )
        .unwrap();

    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();

    // Bump the priority for the test
    let mut job = world.get_component(job_eid, "Job").unwrap().clone();
    job["priority"] = json!(1000);
    world.set_component(job_eid, "Job", job).unwrap();

    let shortage_kinds = vec![];

    let mut job_board = JobBoard::default();
    job_board.update(&world, 1, &shortage_kinds);
    let result = job_board.claim_job(agent_eid, &mut world, 1);
    assert_eq!(result, JobAssignmentResult::Assigned(job_eid));
}

#[test]
fn test_jobs_get_priority_boost_on_resource_shortage_event() {
    use engine_core::systems::job::priority_aging::JobPriorityAgingSystem;

    let mut world = world_helper::make_test_world();

    let stockpile_eid = world.spawn_entity();
    world
        .set_component(
            stockpile_eid,
            "Stockpile",
            json!({ "resources": { "wood": 10, "stone": 10 } }),
        )
        .unwrap();

    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();

    let wood_job_eid = world.spawn_entity();
    world
        .set_component(
            wood_job_eid,
            "Job",
            json!({
                "id": wood_job_eid,
                "job_type": "build",
                "state": "pending",
                "priority": 1,
                "resource_requirements": [{ "kind": "wood", "amount": 5 }],
                "creation_tick": 0,
                "category": "construction"
            }),
        )
        .unwrap();

    let stone_job_eid = world.spawn_entity();
    world
        .set_component(
            stone_job_eid,
            "Job",
            json!({
                "id": stone_job_eid,
                "job_type": "build",
                "state": "pending",
                "priority": 1,
                "resource_requirements": [{ "kind": "stone", "amount": 5 }],
                "creation_tick": 0,
                "category": "construction"
            }),
        )
        .unwrap();

    // Reserve resources so all jobs are runnable
    let mut reservation_system =
        engine_core::systems::job::resource_reservation::ResourceReservationSystem::new();
    reservation_system.run(&mut world);

    // Send the shortage event for "wood"
    world
        .send_event("resource_shortage", json!({ "kind": "wood" }))
        .unwrap();
    world.update_event_buses::<serde_json::Value>();

    // Collect shortage kinds for this tick
    let shortage_kinds = JobPriorityAgingSystem::get_shortage_kinds(&mut world);

    let mut job_board = JobBoard::default();
    job_board.update(&world, 1, &shortage_kinds);
    let result = job_board.claim_job(agent_eid, &mut world, 1);

    assert_eq!(result, JobAssignmentResult::Assigned(wood_job_eid));

    let wood_job = world.get_component(wood_job_eid, "Job").unwrap();
    let stone_job = world.get_component(stone_job_eid, "Job").unwrap();

    let wood_job_effective =
        JobPriorityAgingSystem::compute_effective_priority(wood_job, 1, &shortage_kinds);

    let stone_job_effective =
        JobPriorityAgingSystem::compute_effective_priority(stone_job, 1, &shortage_kinds);

    assert!(
        wood_job_effective > stone_job_effective,
        "Wood job should have received a dynamic priority boost"
    );
}

// --- Section: Scheduling Policies ---

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
    job_board.update(&world, 0, &[]);

    engine_core::systems::job::ai::logic::assign_jobs(&mut world, &mut job_board, 0, &[]);

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
    job_board.update(&world, 0, &[]);

    engine_core::systems::job::ai::logic::assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent_obj = world.get_component(agent, "Agent").unwrap();
    // Should get job2 (the last-added), not job1 (the highest priority)
    assert_eq!(agent_obj["current_job"], job2);
}
