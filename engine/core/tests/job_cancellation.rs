use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::{JobSystem, assign_jobs};
use engine_core::systems::job_board::JobBoard;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn setup_registry() -> Arc<Mutex<engine_core::ecs::registry::ComponentRegistry>> {
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
        name: "Terrain".to_string(),
        schema: serde_json::json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    Arc::new(Mutex::new(registry))
}

#[test]
fn test_job_cancellation_cleans_up_agent_and_emits_event() {
    let registry = setup_registry();
    let mut world = World::new(registry);

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
    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(1);

    world
        .set_component(
            100,
            "Job",
            json!({
                "id": 100,
                "job_type": "dig",
                "status": "pending",
                "cancelled": false,
                "priority": 1
            }),
        )
        .unwrap();
    world.entities.push(100);

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    // Cancel the job
    let mut job = world.get_component(100, "Job").unwrap().clone();
    job["cancelled"] = json!(true);
    world.set_component(100, "Job", job).unwrap();

    // Run job system to process cancellation
    let mut job_system = JobSystem;
    job_system.run(&mut world, None);

    // Agent should be idle and unassigned
    let agent = world.get_component(1, "Agent").unwrap();
    assert!(agent.get("current_job").is_none());
    assert_eq!(agent["state"], "idle");

    // Job should be marked as cancelled
    let job = world.get_component(100, "Job").unwrap();
    assert_eq!(job["status"], "cancelled");

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
            .any(|event| event.get("entity").and_then(|v| v.as_u64()) == Some(100))
    );
}

#[test]
fn test_job_effect_rollback_on_cancel() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Register a reversible effect handler
    {
        let registry = world.effect_processor_registry.take().unwrap();
        registry
            .lock()
            .unwrap()
            .register_handler("ModifyTerrain", |world, eid, effect| {
                // Apply effect: set Terrain to effect["to"]
                let to = effect.get("to").and_then(|v| v.as_str()).unwrap();
                world
                    .set_component(eid, "Terrain", json!({ "kind": to }))
                    .unwrap();
            });
        registry
            .lock()
            .unwrap()
            .register_handler("UndoModifyTerrain", |world, eid, effect| {
                // Rollback effect: set Terrain back to effect["from"]
                let from = effect.get("from").and_then(|v| v.as_str()).unwrap();
                world
                    .set_component(eid, "Terrain", json!({ "kind": from }))
                    .unwrap();
            });
        world.effect_processor_registry = Some(registry);
    }

    // Set up initial terrain
    world
        .set_component(100, "Terrain", json!({ "kind": "rock" }))
        .unwrap();

    // Job with an effect
    world
        .set_component(
            100,
            "Job",
            json!({
                "id": 100,
                "job_type": "dig",
                "status": "pending",
                "cancelled": false,
                "priority": 1
            }),
        )
        .unwrap();

    // Simulate job type registry with effect
    world.job_types.register_job_type(
        "dig",
        vec![json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    );

    // Assign and complete job normally: effect should apply
    {
        let mut job_board = JobBoard::default();
        job_board.update(&world);
        assign_jobs(&mut world, &mut job_board);

        // Mark job as complete
        let mut job = world.get_component(100, "Job").unwrap().clone();
        job["status"] = json!("complete");
        world.set_component(100, "Job", job).unwrap();

        // Run system to apply effect
        let mut job_system = JobSystem;
        job_system.run(&mut world, None);

        let terrain = world.get_component(100, "Terrain").unwrap();
        assert_eq!(terrain["kind"], "rock");
    }

    // Reset for cancellation test
    world
        .set_component(100, "Terrain", json!({ "kind": "rock" }))
        .unwrap();
    world
        .set_component(
            100,
            "Job",
            json!({
                "id": 100,
                "job_type": "dig",
                "status": "pending",
                "cancelled": true,
                "priority": 1
            }),
        )
        .unwrap();

    // Run system: effect should not apply, and rollback (UndoModifyTerrain) should be called
    {
        let mut job_system = JobSystem;
        job_system.run(&mut world, None);

        let terrain = world.get_component(100, "Terrain").unwrap();
        assert_eq!(terrain["kind"], "rock"); // Should remain unchanged
    }
}
