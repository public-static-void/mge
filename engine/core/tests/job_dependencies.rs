use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::{JobSystem, JobTypeRegistry};
use serde_json::json;
use std::sync::{Arc, Mutex};

fn make_test_world_with_job_schema() -> World {
    let schema_dir = "../../engine/assets/schemas";
    let schemas = load_schemas_from_dir(schema_dir).expect("Failed to load schemas");
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

    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "job_type": "dig",
                "status": "pending"
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
                "dependencies": [dep_eid.to_string()]
            }),
        )
        .unwrap();

    let job_types = JobTypeRegistry::default();
    let mut job_system = JobSystem::with_registry(job_types);
    job_system.run(&mut world, None);

    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(main_job_after.get("status").unwrap(), "pending");
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
                "status": "complete"
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
                "dependencies": [dep_eid.to_string()]
            }),
        )
        .unwrap();

    let job_types = JobTypeRegistry::default();
    let mut job_system = JobSystem::with_registry(job_types);
    job_system.run(&mut world, None);

    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_ne!(main_job_after.get("status").unwrap(), "pending");
}
