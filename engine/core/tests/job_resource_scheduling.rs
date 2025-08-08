#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use engine_core::systems::job::resource_reservation::{
    ResourceReservationStatus, ResourceReservationSystem,
};
use serde_json::json;

#[test]
fn test_job_is_assigned_only_if_resources_are_available() {
    let mut world = world_helper::make_test_world();

    // Stockpile with 10 wood
    let stockpile_eid = world.spawn_entity();
    world
        .set_component(
            stockpile_eid,
            "Stockpile",
            json!({ "resources": { "wood": 10 } }),
        )
        .unwrap();

    // Job requires 5 wood
    let job_eid = world.spawn_entity();
    world
        .set_component(
            job_eid,
            "Job",
            json!({
                "id": job_eid,
                "job_type": "build",
                "state": "pending",
                "resource_requirements": [{ "kind": "wood", "amount": 5 }],
                "category": "construction"
            }),
        )
        .unwrap();

    // Agent
    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();

    // Run reservation system before assignment
    let mut reservation_system = ResourceReservationSystem::new();
    reservation_system.run(&mut world, None);

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);

    // Should be able to claim the job
    let result = job_board.claim_job(agent_eid, &mut world, 0);
    assert_eq!(
        result,
        JobAssignmentResult::Assigned(job_eid),
        "Job should be assigned if resources are available"
    );

    // Resources should be reserved
    let status = reservation_system.check_reservation_status(&world, job_eid);
    assert_eq!(
        status,
        ResourceReservationStatus::Reserved,
        "Resources should be reserved after assignment"
    );

    // Release reservation (simulate job completion) BEFORE any further reservation system run
    reservation_system.release_reservation(&mut world, job_eid);

    // Now add a second, identical job
    let job2_eid = world.spawn_entity();
    world
        .set_component(
            job2_eid,
            "Job",
            json!({
                "id": job2_eid,
                "job_type": "build",
                "state": "pending",
                "resource_requirements": [{ "kind": "wood", "amount": 5 }],
                "category": "construction"
            }),
        )
        .unwrap();

    // Run reservation system again so job2 is considered with released resources
    reservation_system.run(&mut world, None);

    job_board.update(&world, 0, &[]);

    let result2 = job_board.claim_job(agent_eid, &mut world, 1);
    assert_eq!(
        result2,
        JobAssignmentResult::Assigned(job2_eid),
        "Second job should be assigned after resources are released"
    );
}

#[test]
fn test_job_remains_pending_if_resources_unavailable() {
    let mut world = world_helper::make_test_world();

    // Stockpile with 2 stone
    let stockpile_eid = world.spawn_entity();
    world
        .set_component(
            stockpile_eid,
            "Stockpile",
            json!({ "resources": { "stone": 2 } }),
        )
        .unwrap();

    // Job requires 3 stone
    let job_eid = world.spawn_entity();
    world
        .set_component(
            job_eid,
            "Job",
            json!({
                "id": job_eid,
                "job_type": "build",
                "state": "pending",
                "resource_requirements": [{ "kind": "stone", "amount": 3 }],
                "category": "construction"
            }),
        )
        .unwrap();

    let mut reservation_system = ResourceReservationSystem::new();
    reservation_system.run(&mut world, None);

    let status = reservation_system.check_reservation_status(&world, job_eid);
    assert_eq!(
        status,
        ResourceReservationStatus::WaitingForResources,
        "Job should remain pending if resources are unavailable"
    );
}

#[test]
fn test_resources_are_released_on_job_cancellation() {
    let mut world = world_helper::make_test_world();

    // Stockpile with 8 iron
    let stockpile_eid = world.spawn_entity();
    world
        .set_component(
            stockpile_eid,
            "Stockpile",
            json!({ "resources": { "iron": 8 } }),
        )
        .unwrap();

    // Job requires 8 iron
    let job_eid = world.spawn_entity();
    world
        .set_component(
            job_eid,
            "Job",
            json!({
                "id": job_eid,
                "job_type": "forge",
                "state": "pending",
                "resource_requirements": [{ "kind": "iron", "amount": 8 }],
                "category": "construction"
            }),
        )
        .unwrap();

    let mut reservation_system = ResourceReservationSystem::new();
    reservation_system.run(&mut world, None);

    // Assign job and reserve resources
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();
    let result = job_board.claim_job(agent_eid, &mut world, 0);
    assert_eq!(
        result,
        JobAssignmentResult::Assigned(job_eid),
        "Job should be assigned if resources are available"
    );

    // Cancel the job
    let mut job = world.get_component(job_eid, "Job").unwrap().clone();
    job["state"] = json!("cancelled");
    world.set_component(job_eid, "Job", job).unwrap();

    // Release reservation
    reservation_system.release_reservation(&mut world, job_eid);

    // Resources should be available again
    let stockpile = world.get_component(stockpile_eid, "Stockpile").unwrap();
    assert_eq!(
        stockpile["resources"]["iron"], 8,
        "Resources should be available again after job cancellation"
    );
}
