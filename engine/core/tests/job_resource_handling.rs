#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::JobSystem;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::resource_reservation::ResourceReservationSystem;
use engine_core::systems::movement_system::MovementSystem;
use serde_json::json;
use world_helper::make_test_world;

#[test]
fn test_agent_fetches_and_delivers_resources_for_job() {
    let mut world = make_test_world();
    world.current_mode = "colony".to_string();

    // --- Map setup using apply_generated_map ---
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

    // Setup: one agent, one stockpile with wood, one job requiring wood
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

    let stockpile_id = world.spawn_entity();
    world
        .set_component(
            stockpile_id,
            "Stockpile",
            json!({
                "resources": { "wood": 10 }
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

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "build",
                "status": "pending",
                "phase": "pending",
                "category": "construction",
                "target_position": {
                    "pos": { "Square": { "x": 2, "y": 0, "z": 0 } }
                },
                "resource_requirements": [
                    { "kind": "wood", "amount": 5 }
                ]
            }),
        )
        .unwrap();

    // Register systems
    world.register_system(ResourceReservationSystem::new());
    world.register_system(JobSystem::new());
    world.register_system(MovementSystem);

    // Tick 1: Reservation should occur, job assigned, phase set to fetching_resources
    world.run_system("ResourceReservationSystem", None).unwrap();

    // Assign the job using JobBoard logic
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    let _ = job_board.claim_job(agent_id, &mut world, 0);

    world.run_system("JobSystem", None).unwrap();

    let job = world.get_component(job_id, "Job").unwrap();
    assert_eq!(job["phase"], "fetching_resources");
    assert_eq!(job["reserved_stockpile"], stockpile_id);

    // Tick 2: Agent starts moving toward stockpile
    world.run_system("MovementSystem", None).unwrap();
    world.run_system("JobSystem", None).unwrap();

    // Tick 3: Agent arrives at stockpile
    world.run_system("MovementSystem", None).unwrap();
    world.run_system("JobSystem", None).unwrap();

    let agent_pos = world.get_component(agent_id, "Position").unwrap();
    let pos = agent_pos.get("pos").unwrap().get("Square").unwrap();
    assert_eq!(pos["x"], 1);
    assert_eq!(pos["y"], 0);

    // Tick 4: Agent picks up wood, phase set to delivering_resources
    world.run_system("JobSystem", None).unwrap();
    let agent = world.get_component(agent_id, "Agent").unwrap();
    let carried = agent.get("carried_resources").unwrap().as_array().unwrap();
    assert_eq!(carried[0]["kind"], "wood");
    assert_eq!(carried[0]["amount"], 5);

    let job = world.get_component(job_id, "Job").unwrap();
    assert_eq!(job["phase"], "delivering_resources");

    // Tick 5: Agent moves to job site
    world.run_system("MovementSystem", None).unwrap();
    world.run_system("JobSystem", None).unwrap();

    // Tick 6: Agent arrives at job site
    world.run_system("MovementSystem", None).unwrap();
    world.run_system("JobSystem", None).unwrap();

    let agent_pos = world
        .get_component(agent_id, "Position")
        .unwrap()
        .get("pos")
        .unwrap()
        .get("Square")
        .unwrap();
    assert_eq!(agent_pos["x"], 2);
    assert_eq!(agent_pos["y"], 0);

    // Tick 7: Agent delivers wood, job phase becomes in_progress
    world.run_system("JobSystem", None).unwrap();
    let job = world.get_component(job_id, "Job").unwrap();
    assert_eq!(job["phase"], "in_progress");
    let delivered = job.get("delivered_resources").unwrap().as_array().unwrap();
    assert_eq!(delivered[0]["kind"], "wood");
    assert_eq!(delivered[0]["amount"], 5);

    // Tick 8+: Job progresses and completes as normal
    for _ in 0..5 {
        world.run_system("JobSystem", None).unwrap();
    }
    let job = world.get_component(job_id, "Job").unwrap();
    assert_eq!(job["status"], "complete");
}
