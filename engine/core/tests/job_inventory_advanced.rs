#[path = "helpers/inventory.rs"]
mod inventory_helper;
#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::JobSystem;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use engine_core::systems::job::resource_reservation::ResourceReservationSystem;
use engine_core::systems::movement_system::MovementSystem;
use inventory_helper::create_inventory;
use serde_json::json;
use world_helper::make_test_world;

#[test]
fn test_agent_makes_multiple_trips_for_large_job() {
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

    // Reserve resources and assign job
    world.run_system("ResourceReservationSystem", None).unwrap();
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assert_eq!(
        job_board.claim_job(agent_id, &mut world, 0),
        JobAssignmentResult::Assigned(job_id)
    );

    // Run simulation ticks: agent should make multiple trips to deliver all wood
    let mut delivered_total = 0;
    let mut trips = 0;
    let mut completed = false;
    for _tick in 0..50 {
        world.run_system("MovementSystem", None).unwrap();
        world.run_system("JobSystem", None).unwrap();

        let job = world.get_component(job_id, "Job").unwrap();
        if let Some(delivered) = job.get("delivered_resources") {
            if let Some(arr) = delivered.as_array() {
                if let Some(wood) = arr.iter().find(|r| r.get("kind") == Some(&json!("wood"))) {
                    delivered_total = wood.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
                }
            }
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

    world.run_system("ResourceReservationSystem", None).unwrap();
    job_board.update(&world);
    assert_eq!(
        job_board.claim_job(agent_id, &mut world, 0),
        JobAssignmentResult::Assigned(job_id2)
    );

    let mut delivered_total2 = 0;
    let mut trips2 = 0;
    let mut completed2 = false;
    for _tick in 0..50 {
        world.run_system("MovementSystem", None).unwrap();
        world.run_system("JobSystem", None).unwrap();

        let job = world.get_component(job_id2, "Job").unwrap();
        if let Some(delivered) = job.get("delivered_resources") {
            if let Some(arr) = delivered.as_array() {
                if let Some(wood) = arr.iter().find(|r| r.get("kind") == Some(&json!("wood"))) {
                    delivered_total2 = wood.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
                }
            }
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
