#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::JobSystem;
use serde_json::json;

#[test]
fn test_job_with_unfinished_dependency_remains_pending() {
    let mut world = world_helper::make_test_world();
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

    // Third tick: dependency should now be "complete"
    job_system.run(&mut world, None);

    // Fourth tick: main job can now advance
    job_system.run(&mut world, None);
    let main_job_after4 = world.get_component(main_eid, "Job").unwrap();
    assert_ne!(main_job_after4.get("status").unwrap(), "pending");
}

#[test]
fn test_job_with_completed_dependency_can_start() {
    let mut world = world_helper::make_test_world();
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
