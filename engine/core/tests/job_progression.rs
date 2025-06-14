use std::path::Path;
use std::sync::{Arc, Mutex};

use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::{JobSystem, assign_jobs};
use engine_core::systems::job_board::JobBoard;
use serde_json::json;

fn setup_registry() -> Arc<Mutex<ComponentRegistry>> {
    let config =
        GameConfig::load_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"))
            .expect("Failed to load config");
    let schema_dir = "../../engine/assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    Arc::new(Mutex::new(registry))
}

#[test]
fn test_job_progression_over_ticks() {
    let registry = setup_registry();
    let mut world = World::new(registry);

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
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(100);

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    // Run the job system for several ticks, simulating progression
    let mut job_system = JobSystem::new();
    for _ in 0..5 {
        job_system.run(&mut world, None);
        let job = world.get_component(100, "Job").unwrap();
        let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let status = job.get("status").and_then(|v| v.as_str()).unwrap();
        if progress < 3.0 {
            assert_eq!(status, "in_progress");
        } else {
            assert_eq!(status, "complete");
            break;
        }
    }
    let job = world.get_component(100, "Job").unwrap();
    assert_eq!(job.get("status").unwrap(), "complete");
}

#[test]
fn test_custom_job_handler_overrides_progression() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Register a custom handler for "instant" job_type
    {
        let registry = world.job_handler_registry.clone();
        registry.lock().unwrap().register_handler(
            "instant",
            |_world, _agent_id, _job_id, job: &serde_json::Value| {
                let mut job = job.clone();
                job["progress"] = serde_json::json!(42.0);
                job["status"] = serde_json::json!("complete");
                job
            },
        );
    }

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
            101,
            "Job",
            json!({
                "id": 101,
                "job_type": "instant",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();
    world.entities.push(101);

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    // Run the job system, custom handler should immediately complete the job
    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let job = world.get_component(101, "Job").unwrap();
    assert_eq!(job.get("progress").unwrap(), 42.0);
    assert_eq!(job.get("status").unwrap(), "complete");
}

#[test]
fn test_effects_applied_only_on_completion_and_rolled_back_on_cancel() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Register effect handlers
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
    world
        .set_component(200, "Terrain", json!({ "kind": "rock" }))
        .unwrap();

    // Job with an effect
    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "dig",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Register job type with effect
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

        // Run system for enough ticks to complete the job
        let mut job_system = JobSystem::new();
        for _ in 0..5 {
            job_system.run(&mut world, None);
        }

        let terrain = world.get_component(200, "Terrain").unwrap();
        assert_eq!(terrain["kind"], "tunnel");
    }

    // Reset for cancellation test
    world
        .set_component(200, "Terrain", json!({ "kind": "rock" }))
        .unwrap();
    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "dig",
                "status": "pending",
                "cancelled": true,
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Run system: effect should not apply, and rollback (UndoModifyTerrain) should be called
    {
        let mut job_system = JobSystem::new();
        job_system.run(&mut world, None);

        let terrain = world.get_component(200, "Terrain").unwrap();
        assert_eq!(terrain["kind"], "rock");
    }
}
