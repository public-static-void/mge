use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::JobSystem;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn make_test_world_with_job_schema() -> World {
    let config = engine_core::config::GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schema_dir = "../../engine/assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    World::new(registry)
}

#[test]
fn job_with_failed_dependency_fails() {
    let mut world = make_test_world_with_job_schema();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    // Dependency fails
    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "job_type": "dig",
                "status": "failed",
                "category": "mining"
            }),
        )
        .unwrap();

    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "job_type": "build",
                "status": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction"
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(main_job_after.get("status").unwrap(), "failed");
}

#[test]
fn job_with_cancelled_dependency_cancels() {
    let mut world = make_test_world_with_job_schema();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    // Dependency cancelled
    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "job_type": "dig",
                "status": "cancelled",
                "category": "mining"
            }),
        )
        .unwrap();

    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "job_type": "build",
                "status": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction"
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(main_job_after.get("status").unwrap(), "cancelled");
}

#[test]
fn job_spawns_child_on_dependency_failure() {
    let mut world = make_test_world_with_job_schema();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    // Dependency fails
    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "job_type": "dig",
                "status": "failed",
                "category": "mining"
            }),
        )
        .unwrap();

    // Main job should spawn a child job if dependency fails
    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "job_type": "build",
                "status": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction",
                "on_dependency_failed_spawn": [{
                    "job_type": "notify",
                    "status": "pending",
                    "category": "notification"
                }]
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    // Main job should be failed, and a child job should have been spawned
    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(main_job_after.get("status").unwrap(), "failed");
    let children = main_job_after
        .get("children")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].get("job_type").unwrap(), "notify");
    assert_eq!(children[0].get("status").unwrap(), "pending");
    assert_eq!(children[0].get("category").unwrap(), "notification");
}
