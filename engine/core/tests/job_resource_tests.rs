#[path = "helpers/world.rs"]
mod world_helper;

#[path = "helpers/inventory.rs"]
mod inventory_helper;
use inventory_helper::create_inventory;

#[path = "helpers/agent.rs"]
mod agent_helper;
use agent_helper::AgentTestHelpers;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use engine_core::systems::job::resource_reservation::{ResourceReservationStatus, ResourceReservationSystem};
use engine_core::systems::job::{JobLogicKind, JobSystem, JobTypeData, assign_jobs};
use engine_core::systems::movement_system::MovementSystem;
use serde_json::json;
use world_helper::make_test_world;

// --- Section: Resource Flow ---

#[test]
fn test_agent_fetches_and_delivers_resources_with_failure_and_interruption() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();
    world.current_mode = "colony".to_string();

    // Map setup
    let map_json = serde_json::json!({
        "topology": "square",
        "width": 3,
        "height": 1,
        "z_levels": 1,
        "cells": [
            { "x": 0, "y": 0, "z": 0, "walkable": true },
            { "x": 1, "y": 0, "z": 0, "walkable": true },
            { "x": 2, "y": 0, "z": 0, "walkable": true }
        ]
    });
    world.apply_generated_map(&map_json).unwrap();

    // Agent setup
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle"
            }),
        )
        .unwrap();
    world
        .set_component(
            agent_id,
            "Position",
            json!({
                "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();
    // Add Inventory to the agent for robust pickup logic
    world
        .set_component(
            agent_id,
            "Inventory",
            json!({
                "max_weight": 100.0,
                "max_slots": 10,
                "max_volume": 100.0,
                "weight": 0.0,
                "slots": [],
                "volume": 0.0
            }),
        )
        .unwrap();

    // Stockpile setup
    let stockpile_id = world.spawn_entity();
    world
        .set_component(
            stockpile_id,
            "Stockpile",
            json!({
                "resources": { "wood": 2 }
            }),
        )
        .unwrap();
    world
        .set_component(
            stockpile_id,
            "Position",
            json!({
                "pos": { "Square": { "x": 1, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();

    // Job setup
    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "build",
                "state": "pending",
                "category": "construction",
                "target_position": {
                    "pos": { "Square": { "x": 2, "y": 0, "z": 0 } }
                },
                "resource_requirements": [
                    { "kind": "wood", "amount": 2 }
                ]
            }),
        )
        .unwrap();

    // Register systems
    world.register_system(ResourceReservationSystem::new());
    world.register_system(JobSystem::new());
    world.register_system(MovementSystem);

    // Reserve resources and assign job
    world.run_system("ResourceReservationSystem").unwrap();
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    match job_board.claim_job(agent_id, &mut world, 0) {
        JobAssignmentResult::Assigned(_) => (),
        other => panic!("Job assignment failed: {other:?}"),
    }
    world.run_system("JobSystem").unwrap();

    // Confirm job state is fetching_resources
    {
        let job = world.get_component(job_id, "Job").unwrap();
        assert_eq!(job["state"], "fetching_resources");
        assert_eq!(job["reserved_stockpile"], stockpile_id);
    }

    // Run ticks until agent picks up resources
    let mut picked_up = false;
    for _tick in 0..20 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();

        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job = world.get_component(job_id, "Job").unwrap();

        if job.get("state") == Some(&json!("delivering_resources")) {
            let carried = agent.get("carried_resources").unwrap().as_array().unwrap();
            assert_eq!(carried[0]["kind"], "wood");
            assert_eq!(carried[0]["amount"], 2);
            picked_up = true;
            break;
        }
    }
    assert!(picked_up, "Agent never picked up resources for delivery");

    // Run ticks until job in_progress (delivered)
    let mut delivered = false;
    for _tick in 0..20 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();

        let job = world.get_component(job_id, "Job").unwrap();

        if job.get("state") == Some(&json!("in_progress")) {
            let delivered_res = job.get("delivered_resources").unwrap().as_array().unwrap();
            assert_eq!(delivered_res[0]["kind"], "wood");
            assert_eq!(delivered_res[0]["amount"], 2);
            delivered = true;
            break;
        }
    }
    assert!(delivered, "Agent never delivered resources");

    // Run ticks until job complete
    let mut completed = false;
    for _tick in 0..10 {
        world.run_system("JobSystem").unwrap();
        let job = world.get_component(job_id, "Job").unwrap();
        if job.get("state") == Some(&json!("complete")) {
            completed = true;
            break;
        }
    }
    assert!(completed, "Job did not complete");

    // --- Now test cancellation with agent carrying resources ---

    // Setup second job
    let job_id2 = world.spawn_entity();
    world
        .set_component(
            job_id2,
            "Job",
            json!({
                "id": job_id2,
                "job_type": "build",
                "state": "pending",
                "category": "construction",
                "target_position": {
                    "pos": { "Square": { "x": 2, "y": 0, "z": 0 } }
                },
                "resource_requirements": [
                    { "kind": "wood", "amount": 2 }
                ]
            }),
        )
        .unwrap();

    // Reset stockpile resources
    world
        .set_component(
            stockpile_id,
            "Stockpile",
            json!({
                "resources": { "wood": 2 }
            }),
        )
        .unwrap();

    world.run_system("ResourceReservationSystem").unwrap();
    job_board.update(&world, 0, &[]);
    match job_board.claim_job(agent_id, &mut world, 0) {
        JobAssignmentResult::Assigned(_) => (),
        other => panic!("Job assignment failed: {other:?}"),
    }
    world.run_system("JobSystem").unwrap();

    // Run ticks until agent picks up resources for second job
    let mut picked_up2 = false;
    for _tick in 0..20 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();

        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job = world.get_component(job_id2, "Job").unwrap();

        if job.get("state") == Some(&json!("delivering_resources")) {
            let carried = agent.get("carried_resources").unwrap().as_array().unwrap();
            assert_eq!(carried[0]["kind"], "wood");
            assert_eq!(carried[0]["amount"], 2);
            picked_up2 = true;
            break;
        }
    }
    assert!(picked_up2, "Agent never picked up resources for second job");

    // Cancel the job
    {
        let mut job = world.get_component(job_id2, "Job").unwrap().clone();
        job["state"] = json!("cancelled");
        world.set_component(job_id2, "Job", job).unwrap();
    }

    // Poll for loose item dropped on cancellation (do NOT check position, only kind/amount/loose)
    let mut found_loose_item = false;
    for _ in 0..5 {
        world.run_system("JobSystem").unwrap();

        for eid in world.get_entities_with_component("Item") {
            let item = world.get_component(eid, "Item").unwrap();
            let _item_pos = world.get_component(eid, "Position").unwrap();
            if item.get("kind") == Some(&json!("wood"))
                && item.get("amount") == Some(&json!(2))
                && item.get("loose") == Some(&json!(true))
            {
                found_loose_item = true;
                break;
            }
        }
        if found_loose_item {
            break;
        }
    }
    assert!(
        found_loose_item,
        "Agent should have dropped wood as a loose item on cancellation"
    );

    let agent = world.get_component(agent_id, "Agent").unwrap();

    assert!(
        agent.get("carried_resources").is_none()
            || agent
                .get("carried_resources")
                .map(|v| v.as_array().map(|a| a.is_empty()).unwrap_or(false))
                .unwrap_or(false),
        "Agent should have dropped resources on cancellation"
    );
}

// --- Section: Resource Scheduling ---

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
    reservation_system.run(&mut world);

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
    reservation_system.run(&mut world);

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
    reservation_system.run(&mut world);

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
    reservation_system.run(&mut world);

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

// --- Section: Inventory Advanced ---

#[test]
fn test_agent_makes_multiple_trips_for_large_job() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();
    world.current_mode = "colony".to_string();

    // Map setup
    let map_json = serde_json::json!({
        "topology": "square",
        "width": 3,
        "height": 1,
        "z_levels": 1,
        "cells": [
            { "x": 0, "y": 0, "z": 0, "walkable": true },
            { "x": 1, "y": 0, "z": 0, "walkable": true },
            { "x": 2, "y": 0, "z": 0, "walkable": true }
        ]
    });
    world.apply_generated_map(&map_json).unwrap();

    // Agent setup: can only carry 3 wood at a time
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle"
            }),
        )
        .unwrap();
    world
        .set_component(
            agent_id,
            "Position",
            json!({
                "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();
    // Give agent an inventory with max_weight = 3.0, empty at start
    let inventory_id = create_inventory(
        &mut world,
        vec![],
        10,    // max_slots
        0.0,   // weight
        3.0,   // max_weight
        0.0,   // volume
        100.0, // max_volume
    );
    world
        .set_component(
            agent_id,
            "Inventory",
            world
                .get_component(inventory_id, "Inventory")
                .unwrap()
                .clone(),
        )
        .unwrap();

    // Stockpile setup: has 7 wood
    let stockpile_id = world.spawn_entity();
    world
        .set_component(
            stockpile_id,
            "Stockpile",
            json!({
                "resources": { "wood": 7 }
            }),
        )
        .unwrap();
    world
        .set_component(
            stockpile_id,
            "Position",
            json!({
                "pos": { "Square": { "x": 1, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();

    // Job setup: requires 7 wood
    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "build",
                "state": "pending",
                "category": "construction",
                "target_position": {
                    "pos": { "Square": { "x": 2, "y": 0, "z": 0 } }
                },
                "resource_requirements": [
                    { "kind": "wood", "amount": 7 }
                ]
            }),
        )
        .unwrap();

    // Register systems
    world.register_system(ResourceReservationSystem::new());
    world.register_system(JobSystem::new());
    world.register_system(MovementSystem);

    // Register job handler for "build" jobs that only completes when all resources are delivered
    {
        let registry = world.job_handler_registry.clone();
        registry.lock().unwrap().register_handler(
            "build",
            move |_world, _agent_id, _job_id, job| {
                let mut job = job.clone();
                let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                if matches!(
                    state,
                    "failed" | "complete" | "cancelled" | "interrupted" | "paused"
                ) {
                    return job;
                }
                let mut progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);

                let mut all_delivered = true;
                if let Some(reqs) = job.get("resource_requirements").and_then(|v| v.as_array()) {
                    if let Some(delivered) =
                        job.get("delivered_resources").and_then(|v| v.as_array())
                    {
                        for req in reqs {
                            let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                            let amount = req.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
                            let delivered_amt = delivered
                                .iter()
                                .find(|r| r.get("kind").and_then(|v| v.as_str()) == Some(kind))
                                .and_then(|r| r.get("amount").and_then(|v| v.as_i64()))
                                .unwrap_or(0);
                            if delivered_amt < amount {
                                all_delivered = false;
                                break;
                            }
                        }
                    } else {
                        all_delivered = false;
                    }
                }

                if all_delivered {
                    progress += 1.0;
                    job["progress"] = json!(progress);

                    if progress >= 3.0 {
                        job["state"] = json!("complete");
                    }
                }
                job
            },
        );
    }

    // Reserve resources and assign job
    world.run_system("ResourceReservationSystem").unwrap();
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);

    assert_eq!(
        job_board.claim_job(agent_id, &mut world, 0),
        JobAssignmentResult::Assigned(job_id)
    );

    // Run simulation ticks: agent should make multiple trips to deliver all wood
    let mut delivered_total = 0;
    let mut trips = 0;
    let mut completed = false;
    for _tick in 0..50 {
        world.run_system("ResourceReservationSystem").unwrap();
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();

        let job = world.get_component(job_id, "Job").unwrap();
        let _agent = world.get_component(agent_id, "Agent").unwrap();

        if let Some(delivered) = job.get("delivered_resources")
            && let Some(arr) = delivered.as_array()
            && let Some(wood) = arr.iter().find(|r| r.get("kind") == Some(&json!("wood")))
        {
            delivered_total = wood.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
        }
        if job.get("state") == Some(&json!("delivering_resources")) {
            trips += 1;
        }
        if job.get("state") == Some(&json!("complete")) {
            completed = true;
            break;
        }
    }

    assert!(completed, "Job did not complete");
    assert_eq!(delivered_total, 7, "Not all wood delivered");
    assert!(
        trips >= 3,
        "Agent should have made multiple trips (at least 3 for 3+3+1)"
    );

    // Now increase agent's max_weight and verify fewer trips are needed
    let mut new_inventory = world.get_component(agent_id, "Inventory").unwrap().clone();
    new_inventory["max_weight"] = json!(7.0);
    world
        .set_component(agent_id, "Inventory", new_inventory)
        .unwrap();

    // Reset job and stockpile for new run
    let job_id2 = world.spawn_entity();
    world
        .set_component(
            job_id2,
            "Job",
            json!({
                "id": job_id2,
                "job_type": "build",
                "state": "pending",
                "category": "construction",
                "target_position": {
                    "pos": { "Square": { "x": 2, "y": 0, "z": 0 } }
                },
                "resource_requirements": [
                    { "kind": "wood", "amount": 7 }
                ]
            }),
        )
        .unwrap();
    world
        .set_component(
            stockpile_id,
            "Stockpile",
            json!({
                "resources": { "wood": 7 }
            }),
        )
        .unwrap();

    if let Some(stockpile) = world.get_component(stockpile_id, "Stockpile") {
        println!("Stockpile resources: {:?}", stockpile.get("resources"));
    } else {
        println!("Stockpile {stockpile_id} not found.");
    }

    world.run_system("ResourceReservationSystem").unwrap();
    job_board.update(&world, 0, &[]);
    assert_eq!(
        job_board.claim_job(agent_id, &mut world, 0),
        JobAssignmentResult::Assigned(job_id2)
    );

    let mut delivered_total2 = 0;
    let mut trips2 = 0;
    let mut completed2 = false;
    for _tick in 0..50 {
        world.run_system("ResourceReservationSystem").unwrap();
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();

        let job = world.get_component(job_id2, "Job").unwrap();
        let _agent = world.get_component(agent_id, "Agent").unwrap();

        if let Some(delivered) = job.get("delivered_resources")
            && let Some(arr) = delivered.as_array()
            && let Some(wood) = arr.iter().find(|r| r.get("kind") == Some(&json!("wood")))
        {
            delivered_total2 = wood.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
        }
        if job.get("state") == Some(&json!("delivering_resources")) {
            trips2 += 1;
        }
        if job.get("state") == Some(&json!("complete")) {
            completed2 = true;
            break;
        }
    }
    assert!(completed2, "Job did not complete (second run)");
    assert_eq!(delivered_total2, 7, "Not all wood delivered (second run)");
    assert!(
        trips2 <= 2,
        "Agent should have made fewer trips with higher capacity"
    );
}

// --- Section: Effects ---

/// Verifies that job effects are processed when a job completes.
/// This test registers a job type with an effect that modifies terrain,
/// assigns the job to an entity, and ensures the effect is applied on completion.
#[test]
fn test_job_effects_are_processed_on_completion() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register an effect handler for "ModifyTerrain" that sets the Terrain type.
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("ModifyTerrain", |world, eid, effect| {
            if let Some(to) = effect.get("to").and_then(|v| v.as_str()) {
                world
                    .set_component(eid, "Terrain", json!({ "type": to, "kind": to }))
                    .unwrap();
            }
        });

    // Register a job type with an effect that modifies terrain.
    let job_type_data = JobTypeData {
        name: "DigTunnel".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    };
    world.job_types.register(
        job_type_data,
        JobLogicKind::Native(|_, _, _, job| job.clone()),
    );

    // Create an entity and assign the job.
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Job",
            json!({
                "id": eid, // <-- Make sure the job has an id!
                "job_type": "DigTunnel",
                "state": "pending",
                "category": "test",
                "progress": 0.0
            }),
        )
        .unwrap();

    // Spawn an idle agent so the job can be assigned and completed.
    world.spawn_idle_agent();

    // Run the job system enough times to complete the job and process effects.
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();
    for _ in 0..4 {
        job_board.update(&world, 0, &[]);
        engine_core::systems::job::ai::logic::assign_jobs(&mut world, &mut job_board, 0, &[]);
        job_system.run(&mut world);
    }

    // After job completion, the terrain type should be updated by the effect.
    let terrain = world.get_component(eid, "Terrain").unwrap();
    assert_eq!(
        terrain["type"], "tunnel",
        "Terrain type should be 'tunnel' after job completion"
    );
}

// --- Section: Chained Effects ---

#[test]
fn test_job_chained_effects() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register minimal schemas for test components in "colony" mode
    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(
            &json!({
                "title": "FirstApplied",
                "type": "object",
                "properties": { "value": { "type": "integer" } },
                "required": ["value"],
                "modes": ["colony"]
            })
            .to_string(),
        )
        .unwrap();

    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(
            &json!({
                "title": "SecondApplied",
                "type": "object",
                "properties": { "value": { "type": "integer" } },
                "required": ["value"],
                "modes": ["colony"]
            })
            .to_string(),
        )
        .unwrap();

    // Register the JobSystem
    world.systems.register_system(JobSystem::new());

    // Register effect handlers
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("first", |world, eid, effect| {
            world
                .set_component(eid, "FirstApplied", json!({"value": effect["value"]}))
                .unwrap();
        });

    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("second", |world, eid, effect| {
            world
                .set_component(eid, "SecondApplied", json!({"value": effect["value"]}))
                .unwrap();
        });

    // Register job type with a chained effect (schema-driven)
    let job_type_data = JobTypeData {
        name: "ChainedJob".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![serde_json::json!({
            "action": "first",
            "value": 1,
            "effects": [
                { "action": "second", "value": 2 }
            ]
        })],
    };
    world.job_types.register(
        job_type_data,
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("complete");
            }
            job
        }),
    );

    // Create an agent with the correct specialization
    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "skills": {},
                "preferences": {},
                "state": "idle",
                "specializations": ["test"]
            }),
        )
        .unwrap();

    // Create the job and assign the agent
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Job",
            json!({
                "job_type": "ChainedJob",
                "state": "in_progress",
                "progress": 0.0,
                "category": "test",
                "assigned_to": agent
            }),
        )
        .unwrap();

    // Run the job system enough times to complete the job
    for _ in 0..4 {
        world.run_system("JobSystem").unwrap();
    }

    let first = world.get_component(eid, "FirstApplied").unwrap();
    assert_eq!(first["value"], 1, "First effect should be applied");

    let second = world.get_component(eid, "SecondApplied").unwrap();
    assert_eq!(
        second["value"], 2,
        "Second (chained) effect should be applied"
    );
}

// --- Section: Conditional Effects ---

#[test]
fn test_job_effect_with_world_state_condition() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register the JobSystem
    world.systems.register_system(JobSystem::new());

    // Register a simple effect handler for "ModifyTerrain"
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("ModifyTerrain", |world, eid, effect| {
            let to = effect
                .get("to")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            world
                .set_component(eid, "Terrain", json!({ "kind": to }))
                .unwrap();
        });

    // Register job type with a conditional effect
    let job_type_data = JobTypeData {
        name: "ConditionalDig".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel",
            "condition": { "world_state": { "resource": "medkits", "gte": 1 } }
        })],
    };
    world.job_types.register(
        job_type_data,
        // ADVANCE progress and complete the job!
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("complete");
            }
            job
        }),
    );

    // Create an agent with the correct specialization
    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "skills": {},
                "preferences": {},
                "state": "idle",
                "specializations": ["test"]
            }),
        )
        .unwrap();

    // Create the job and assign the agent
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Terrain", json!({ "kind": "rock" }))
        .unwrap();
    world
        .set_component(
            eid,
            "Job",
            json!({
                "job_type": "ConditionalDig",
                "state": "in_progress",
                "progress": 0.0,
                "category": "test",
                "assigned_to": agent
            }),
        )
        .unwrap();

    // Run the job system enough times to complete the job (no medkits yet)
    for _ in 0..4 {
        world.run_system("JobSystem").unwrap();
    }

    let terrain = world.get_component(eid, "Terrain").unwrap();
    assert_eq!(
        terrain["kind"], "rock",
        "Terrain kind should remain 'rock' when condition is not met"
    );

    // Now add medkits to the world
    world.set_global_resource_amount("medkits", 2.0);

    // Reset job to pending, assign agent, and run again
    world
        .set_component(
            eid,
            "Job",
            json!({
                "job_type": "ConditionalDig",
                "state": "in_progress",
                "progress": 0.0,
                "category": "test",
                "assigned_to": agent
            }),
        )
        .unwrap();

    for _ in 0..4 {
        world.run_system("JobSystem").unwrap();
    }

    let terrain = world.get_component(eid, "Terrain").unwrap();
    assert_eq!(
        terrain["kind"], "tunnel",
        "Terrain kind should become 'tunnel' when condition is met"
    );
}

// --- Section: Conditional Spawning ---

/// Test that a conditional child job is spawned when the parent fails.
/// Also verifies correct assignment and state transitions.
#[test]
fn test_conditional_child_spawn_on_failure() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register robust, terminal-aware handler for "main"
    let main_job_type = JobTypeData {
        name: "main".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        main_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
                return job;
            }
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("failed"); // intentionally fail to test child spawn
            }
            job
        }),
    );

    // Register robust, terminal-aware handler for "repair"
    let repair_job_type = JobTypeData {
        name: "repair".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        repair_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
                return job;
            }
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("complete");
            }
            job
        }),
    );

    // Register robust, terminal-aware handler for "dep"
    let dep_job_type = JobTypeData {
        name: "dep".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        dep_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| job.clone()),
    );

    // Create a dependency job that is already failed.
    let dep_id = world.spawn_entity();
    world
        .set_component(
            dep_id,
            "Job",
            json!({
                "id": dep_id,
                "job_type": "dep",
                "state": "failed",
                "priority": 1,
                "category": "test"
            }),
        )
        .unwrap();

    // Create a parent job with a conditional child that spawns on failure.
    let parent_id = world.spawn_entity();
    world
        .set_component(
            parent_id,
            "Job",
            json!({
                "id": parent_id,
                "job_type": "main",
                "state": "pending",
                "priority": 1,
                "category": "test",
                "dependencies": [dep_id.to_string()],
                "conditional_children": [
                    {
                        "spawn_if": { "field": "state", "equals": "failed" },
                        "job": {
                            "job_type": "repair",
                            "state": "in_progress",
                            "priority": 1,
                            "category": "test"
                        }
                    }
                ]
            }),
        )
        .unwrap();

    // Create an agent with specializations.
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle",
                "specializations": ["test"]
            }),
        )
        .unwrap();

    // Run assignment and orchestrator
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // Set agent back to idle so it can be assigned to the child job.
    let mut agent = world.get_component(agent_id, "Agent").unwrap().clone();
    agent["state"] = serde_json::json!("idle");
    agent["current_job"] = serde_json::Value::Null;
    world.set_component(agent_id, "Agent", agent).unwrap();

    // Assign jobs again so the agent can pick up the spawned child job.
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    // Run the job system again to process the child job.
    job_system.run(&mut world);

    // Check that exactly one child job was spawned.
    let spawned_jobs: Vec<_> = world
        .get_entities_with_component("Job")
        .into_iter()
        .filter(|&eid| eid != parent_id && eid != dep_id)
        .collect();

    assert_eq!(
        spawned_jobs.len(),
        1,
        "Exactly one conditional child job should be spawned"
    );
    let child = world.get_component(spawned_jobs[0], "Job").unwrap();
    assert_eq!(child["job_type"], "repair");
    assert!(
        child["state"] == "pending"
            || child["state"] == "in_progress"
            || child["state"] == "complete",
        "Child job state should be 'pending', 'in_progress', or 'complete', got {:?}",
        child["state"]
    );
}

/// Test that a conditional child job is spawned based on world state.
/// Also verifies correct assignment and state transitions.
#[test]
fn test_conditional_child_spawn_on_world_state() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register robust, terminal-aware handler for "main"
    let main_job_type = JobTypeData {
        name: "main".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        main_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
                return job;
            }
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("complete");
            }
            job
        }),
    );

    // Register robust, terminal-aware handler for "gather_food"
    let gather_food_job_type = JobTypeData {
        name: "gather_food".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![],
    };
    world.job_types.register(
        gather_food_job_type,
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
                return job;
            }
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("complete");
            }
            job
        }),
    );

    // Create the parent job with a conditional child.
    let parent_id = world.spawn_entity();

    world
        .set_component(
            parent_id,
            "Job",
            json!({
                "id": parent_id,
                "job_type": "main",
                "state": "pending",
                "progress": 0.0,
                "priority": 1,
                "category": "test",
                "conditional_children": [
                    {
                        "spawn_if": { "world_state": { "resource": "food", "lte": 10.0 } },
                        "job": {
                            "job_type": "gather_food",
                            "state": "in_progress",
                            "priority": 1,
                            "category": "test"
                        }
                    }
                ]
            }),
        )
        .unwrap();

    // Create an agent with the correct specialization.
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle",
                "specializations": ["test"]
            }),
        )
        .unwrap();

    // Make sure the world state will trigger the conditional child.
    world.set_global_resource_amount("food", 5.0);

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    // Set parent job to in_progress and assigned.
    let mut parent_job = world.get_component(parent_id, "Job").unwrap().clone();
    parent_job["state"] = serde_json::json!("in_progress");
    parent_job["assigned_to"] = serde_json::json!(agent_id);
    world.set_component(parent_id, "Job", parent_job).unwrap();

    let mut job_system = JobSystem::new();

    for _ in 0..5 {
        job_system.run(&mut world);
    }

    // Set agent back to idle so it can be assigned to the child job.
    let mut agent = world.get_component(agent_id, "Agent").unwrap().clone();
    agent["state"] = serde_json::json!("idle");
    agent["current_job"] = serde_json::Value::Null;
    world.set_component(agent_id, "Agent", agent).unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    for _ in 0..5 {
        job_system.run(&mut world);
    }

    // Check that exactly one child job was spawned.
    let spawned_jobs: Vec<_> = world
        .get_entities_with_component("Job")
        .into_iter()
        .filter(|&eid| eid != parent_id)
        .collect();

    assert_eq!(
        spawned_jobs.len(),
        1,
        "Exactly one conditional child job should be spawned"
    );
    let child = world.get_component(spawned_jobs[0], "Job").unwrap();
    assert_eq!(child["job_type"], "gather_food");
    assert!(
        child["state"] == "pending"
            || child["state"] == "in_progress"
            || child["state"] == "complete",
        "Child job state should be 'pending', 'in_progress', or 'complete', got {:?}",
        child["state"]
    );
}
