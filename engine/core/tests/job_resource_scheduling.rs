use engine_core::ecs::world::World;
use engine_core::systems::job::resource_reservation::{
    ResourceReservationStatus, ResourceReservationSystem,
};
use engine_core::systems::job_board::JobBoard;
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
    registry.register_external_schema(engine_core::ecs::schema::ComponentSchema {
        name: "Stockpile".to_string(),
        schema: serde_json::json!({
            "type": "object",
            "properties": {
                "resources": {
                    "type": "object",
                    "additionalProperties": { "type": "number", "minimum": 0 }
                }
            },
            "required": ["resources"]
        }),
        modes: vec!["colony".to_string()],
    });
    let registry = Arc::new(Mutex::new(registry));
    World::new(registry)
}

#[test]
fn job_is_assigned_only_if_resources_are_available() {
    let mut world = setup_world();

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
                "status": "pending",
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
    job_board.update(&world);

    // Should be able to claim the job
    let result = job_board.claim_job(agent_eid, &mut world, 0);
    assert_eq!(
        result,
        engine_core::systems::job_board::JobAssignmentResult::Assigned(job_eid)
    );

    // Resources should be reserved
    let status = reservation_system.check_reservation_status(&world, job_eid);
    assert_eq!(status, ResourceReservationStatus::Reserved);

    // Try to assign a second, identical job (should fail due to insufficient wood)
    let job2_eid = world.spawn_entity();
    world
        .set_component(
            job2_eid,
            "Job",
            json!({
                "id": job2_eid,
                "job_type": "build",
                "status": "pending",
                "resource_requirements": [{ "kind": "wood", "amount": 5 }],
                "category": "construction"
            }),
        )
        .unwrap();

    job_board.update(&world);
    let result2 = job_board.claim_job(agent_eid, &mut world, 1);
    assert_eq!(
        result2,
        engine_core::systems::job_board::JobAssignmentResult::NoJobsAvailable
    );

    // Release reservation (simulate completion)
    reservation_system.release_reservation(&mut world, job_eid);

    // Now, job2 should be assignable
    reservation_system.run(&mut world, None);
    job_board.update(&world);
    let result3 = job_board.claim_job(agent_eid, &mut world, 2);
    assert_eq!(
        result3,
        engine_core::systems::job_board::JobAssignmentResult::Assigned(job2_eid)
    );
}

#[test]
fn job_remains_pending_if_resources_unavailable() {
    let mut world = setup_world();

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
                "status": "pending",
                "resource_requirements": [{ "kind": "stone", "amount": 3 }],
                "category": "construction"
            }),
        )
        .unwrap();

    let mut reservation_system = ResourceReservationSystem::new();
    reservation_system.run(&mut world, None);

    let status = reservation_system.check_reservation_status(&world, job_eid);
    assert_eq!(status, ResourceReservationStatus::WaitingForResources);
}

#[test]
fn resources_are_released_on_job_cancellation() {
    let mut world = setup_world();

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
                "status": "pending",
                "resource_requirements": [{ "kind": "iron", "amount": 8 }],
                "category": "construction"
            }),
        )
        .unwrap();

    let mut reservation_system = ResourceReservationSystem::new();
    reservation_system.run(&mut world, None);

    // Assign job and reserve resources
    let mut job_board = JobBoard::default();
    job_board.update(&world);
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
        engine_core::systems::job_board::JobAssignmentResult::Assigned(job_eid)
    );

    // Cancel the job
    let mut job = world.get_component(job_eid, "Job").unwrap().clone();
    job["status"] = json!("cancelled");
    world.set_component(job_eid, "Job", job).unwrap();

    // Release reservation
    reservation_system.release_reservation(&mut world, job_eid);

    // Resources should be available again
    let stockpile = world.get_component(stockpile_eid, "Stockpile").unwrap();
    assert_eq!(stockpile["resources"]["iron"], 8);
}
