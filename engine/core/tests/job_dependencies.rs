use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::JobSystem;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn make_test_world_with_job_schema() -> World {
    let config = GameConfig::load_from_file(
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
fn job_with_unfinished_dependency_remains_pending() {
    let mut world = make_test_world_with_job_schema();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    // Dependency starts as "pending"
    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending",
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

    // First tick: dependency is advanced, but not yet "complete"
    job_system.run(&mut world, None);

    // Main job should still be pending
    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(main_job_after.get("status").unwrap(), "pending");

    // Second tick: dependency may now be "in_progress"
    job_system.run(&mut world, None);
    let main_job_after2 = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(main_job_after2.get("status").unwrap(), "pending");

    // Third tick: dependency should now be "complete", so main job can advance
    job_system.run(&mut world, None);
    let main_job_after3 = world.get_component(main_eid, "Job").unwrap();
    assert_ne!(main_job_after3.get("status").unwrap(), "pending");
}

#[test]
fn job_with_completed_dependency_can_start() {
    let mut world = make_test_world_with_job_schema();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "job_type": "dig",
                "status": "complete",
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
    assert_ne!(main_job_after.get("status").unwrap(), "pending");
}
