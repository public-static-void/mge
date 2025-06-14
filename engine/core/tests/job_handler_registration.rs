use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::{JobSystem, assign_jobs};
use engine_core::systems::job_board::JobBoard;
use serde_json::json;
use std::path::Path;
use std::sync::{Arc, Mutex};

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
fn test_register_job_handler_api_invokes_handler() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Register a custom handler for "superfast" job_type
    world.register_job_handler(
        "superfast",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            job["progress"] = serde_json::json!(999.0);
            job["status"] = serde_json::json!("complete");
            job
        },
    );

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
            123,
            "Job",
            json!({
                "id": 123,
                "job_type": "superfast",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();
    world.entities.push(123);

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    // Run the job system, custom handler should immediately complete the job
    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let job = world.get_component(123, "Job").unwrap();
    assert_eq!(job.get("progress").unwrap(), 999.0);
    assert_eq!(job.get("status").unwrap(), "complete");
}

#[test]
fn test_register_job_handler_multiple_types() {
    let registry = setup_registry();
    let mut world = World::new(registry);

    // Register handlers for two types
    world.register_job_handler(
        "foo",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            job["progress"] = serde_json::json!(1.0);
            job["status"] = serde_json::json!("complete");
            job
        },
    );
    world.register_job_handler(
        "bar",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            job["progress"] = serde_json::json!(2.0);
            job["status"] = serde_json::json!("complete");
            job
        },
    );

    // Add jobs of both types
    world
        .set_component(
            10,
            "Job",
            json!({
                "id": 10,
                "job_type": "foo",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "foo"
            }),
        )
        .unwrap();
    world.entities.push(10);

    world
        .set_component(
            20,
            "Job",
            json!({
                "id": 20,
                "job_type": "bar",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "bar"
            }),
        )
        .unwrap();
    world.entities.push(20);

    // Run the job system
    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let job_foo = world.get_component(10, "Job").unwrap();
    let job_bar = world.get_component(20, "Job").unwrap();
    assert_eq!(job_foo.get("progress").unwrap(), 1.0);
    assert_eq!(job_foo.get("status").unwrap(), "complete");
    assert_eq!(job_bar.get("progress").unwrap(), 2.0);
    assert_eq!(job_bar.get("status").unwrap(), "complete");
}
