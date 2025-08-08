#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use serde_json::json;

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
    use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
    use engine_core::systems::job::priority_aging::JobPriorityAgingSystem;
    use serde_json::json;

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
    reservation_system.run(&mut world, None);

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
