#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::JobSystem;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use engine_core::systems::job::resource_reservation::ResourceReservationSystem;
use engine_core::systems::movement_system::MovementSystem;
use serde_json::json;
use world_helper::make_test_world;

#[test]
fn test_agent_fetches_and_delivers_resources_with_failure_and_interruption() {
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
    world.run_system("ResourceReservationSystem", None).unwrap();
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    match job_board.claim_job(agent_id, &mut world, 0) {
        JobAssignmentResult::Assigned(_) => (),
        other => panic!("Job assignment failed: {other:?}"),
    }
    world.run_system("JobSystem", None).unwrap();

    // Confirm job state is fetching_resources
    {
        let job = world.get_component(job_id, "Job").unwrap();
        assert_eq!(job["state"], "fetching_resources");
        assert_eq!(job["reserved_stockpile"], stockpile_id);
    }

    // Run ticks until agent picks up resources
    let mut picked_up = false;
    for _tick in 0..20 {
        world.run_system("MovementSystem", None).unwrap();
        world.run_system("JobSystem", None).unwrap();

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
        world.run_system("MovementSystem", None).unwrap();
        world.run_system("JobSystem", None).unwrap();

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
        world.run_system("JobSystem", None).unwrap();
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

    world.run_system("ResourceReservationSystem", None).unwrap();
    job_board.update(&world);
    match job_board.claim_job(agent_id, &mut world, 0) {
        JobAssignmentResult::Assigned(_) => (),
        other => panic!("Job assignment failed: {other:?}"),
    }
    world.run_system("JobSystem", None).unwrap();

    // Run ticks until agent picks up resources for second job
    let mut picked_up2 = false;
    for _tick in 0..20 {
        world.run_system("MovementSystem", None).unwrap();
        world.run_system("JobSystem", None).unwrap();

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
        job["cancelled"] = json!(true);
        world.set_component(job_id2, "Job", job).unwrap();
    }

    // Poll for loose item dropped on cancellation (do NOT check position, only kind/amount/loose)
    let mut found_loose_item = false;
    for _ in 0..5 {
        world.run_system("JobSystem", None).unwrap();

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
