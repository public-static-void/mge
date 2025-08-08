#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, JobTypeData, assign_jobs};
use serde_json::json;

#[test]
fn test_job_cancellation_cleans_up_agent_and_emits_event() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register a dummy effect handler (should not be called on cancel)
    {
        let registry = world.effect_processor_registry.take().unwrap();
        registry
            .lock()
            .unwrap()
            .register_handler("ModifyTerrain", |_world, _eid, _effect| {
                panic!("Effect should not be processed on cancellation");
            });
        world.effect_processor_registry = Some(registry);
    }

    // Agent and job setup
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

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let _agent = world.get_component(agent_id, "Agent").unwrap();
    let _job = world.get_component(job_id, "Job").unwrap();

    // Cancel the job (by setting state to "cancelled")
    let mut job = world.get_component(job_id, "Job").unwrap().clone();
    job["state"] = json!("cancelled");
    world.set_component(job_id, "Job", job).unwrap();

    // Run job system to process cancellation
    let mut job_system = JobSystem;
    job_system.run(&mut world, None);

    let agent = world.get_component(agent_id, "Agent").unwrap();
    let job = world.get_component(job_id, "Job").unwrap();

    // Agent should be idle and unassigned
    assert!(
        agent.get("current_job").is_none_or(|v| v.is_null()),
        "Agent should have no current job after cancellation"
    );
    assert_eq!(
        agent["state"], "idle",
        "Agent should be idle after cancellation"
    );

    // Job should be marked as cancelled
    assert_eq!(job["state"], "cancelled", "Job state should be 'cancelled'");

    // Event should be emitted
    world.update_event_buses::<serde_json::Value>();
    let bus = world
        .get_event_bus::<serde_json::Value>("job_cancelled")
        .unwrap();
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
    assert!(
        events
            .iter()
            .any(|event| event.get("entity").and_then(|v| v.as_u64()) == Some(job_id as u64)),
        "Job cancelled event should be emitted for job"
    );
}

#[test]
fn test_job_effect_rollback_on_cancel() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register a reversible effect handler
    {
        let registry = world.effect_processor_registry.take().unwrap();
        registry
            .lock()
            .unwrap()
            .register_handler("ModifyTerrain", |world, eid, effect| {
                let to = effect.get("to").and_then(|v| v.as_str()).unwrap();
                world
                    .set_component(eid, "Terrain", json!({ "kind": to }))
                    .unwrap();
            });
        registry
            .lock()
            .unwrap()
            .register_handler("UndoModifyTerrain", |world, eid, effect| {
                let from = effect.get("from").and_then(|v| v.as_str()).unwrap();
                world
                    .set_component(eid, "Terrain", json!({ "kind": from }))
                    .unwrap();
            });
        world.effect_processor_registry = Some(registry);
    }

    // Set up initial terrain
    let entity_id = world.spawn_entity();
    world
        .set_component(entity_id, "Terrain", json!({ "kind": "rock" }))
        .unwrap();

    // Job with an effect
    world
        .set_component(
            entity_id,
            "Job",
            json!({
                "id": entity_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Simulate job type registry with effect
    world.job_types.register_job_type(JobTypeData {
        name: "dig".to_string(),
        requirements: vec![],
        duration: None,
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    });

    // Assign and complete job normally: effect should apply
    {
        let mut job_board = JobBoard::default();
        job_board.update(&world, 0, &[]);
        assign_jobs(&mut world, &mut job_board, 0, &[]);

        // Mark job as complete
        let mut job = world.get_component(entity_id, "Job").unwrap().clone();
        job["state"] = json!("complete");
        world.set_component(entity_id, "Job", job).unwrap();

        let _terrain = world.get_component(entity_id, "Terrain").unwrap();

        // Run system to apply effect
        let mut job_system = JobSystem;
        job_system.run(&mut world, None);

        let terrain = world.get_component(entity_id, "Terrain").unwrap();
        assert_eq!(
            terrain["kind"], "tunnel",
            "Terrain should be tunnel after effect"
        );
    }

    // Reset for cancellation test
    world
        .set_component(entity_id, "Terrain", json!({ "kind": "rock" }))
        .unwrap();
    world
        .set_component(
            entity_id,
            "Job",
            json!({
                "id": entity_id,
                "job_type": "dig",
                "state": "cancelled",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    let _terrain = world.get_component(entity_id, "Terrain").unwrap();

    // Run system: effect should not apply, and rollback (UndoModifyTerrain) should be called
    {
        let mut job_system = JobSystem;
        job_system.run(&mut world, None);

        let terrain = world.get_component(entity_id, "Terrain").unwrap();
        assert_eq!(
            terrain["kind"], "rock",
            "Terrain should remain rock after cancellation"
        );
    }
}

#[test]
fn test_job_cancellation_releases_resources_and_cancels_children() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Set up stockpile with resources and a job that reserves them
    let stockpile_id = world.spawn_entity();
    world
        .set_component(
            stockpile_id,
            "Stockpile",
            json!({ "resources": { "wood": 10 } }),
        )
        .unwrap();

    let child_id = world.spawn_entity();
    let parent_id = world.spawn_entity();

    world
        .set_component(
            child_id,
            "Job",
            json!({
                "id": child_id,
                "job_type": "subtask",
                "state": "pending",
                "category": "construction"
            }),
        )
        .unwrap();

    world
        .set_component(
            parent_id,
            "Job",
            json!({
                "id": parent_id,
                "job_type": "build",
                "state": "pending",
                "resource_requirements": [{ "kind": "wood", "amount": 5 }],
                "reserved_resources": [{ "kind": "wood", "amount": 5 }],
                "reserved_stockpile": stockpile_id,
                "category": "construction",
                "children": [
                    {
                        "id": child_id,
                        "job_type": "subtask",
                        "state": "pending",
                        "category": "construction"
                    }
                ]
            }),
        )
        .unwrap();

    let _stockpile = world.get_component(stockpile_id, "Stockpile").unwrap();
    let _parent_job = world.get_component(parent_id, "Job").unwrap();

    // Cancel the parent job (by setting state to "cancelled")
    let mut job = world.get_component(parent_id, "Job").unwrap().clone();
    job["state"] = json!("cancelled");
    world.set_component(parent_id, "Job", job).unwrap();

    // Run job system to process cancellation
    let mut job_system = JobSystem;
    job_system.run(&mut world, None);

    let stockpile = world.get_component(stockpile_id, "Stockpile").unwrap();
    let parent_job = world.get_component(parent_id, "Job").unwrap();

    // Check that resources are released
    assert_eq!(
        stockpile["resources"]["wood"], 10,
        "Resources should be released back to stockpile"
    );

    // Check that child job is cancelled
    let child_id = parent_job
        .get("children")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|child| child.get("id"))
        .and_then(|id| id.as_u64())
        .unwrap() as u32;
    let child_job = world.get_component(child_id, "Job").unwrap();
    assert_eq!(
        child_job["state"], "cancelled",
        "Child job should be cancelled"
    );
}
