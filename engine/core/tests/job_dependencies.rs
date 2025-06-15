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

    job_system.run(&mut world, None);

    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(
        main_job_after.get("status").unwrap(),
        "pending",
        "Main job should remain pending while dependency is unfinished"
    );

    job_system.run(&mut world, None);
    let main_job_after2 = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(
        main_job_after2.get("status").unwrap(),
        "pending",
        "Main job should still be pending while dependency is in progress"
    );

    job_system.run(&mut world, None);
    let main_job_after3 = world.get_component(main_eid, "Job").unwrap();
    assert_ne!(
        main_job_after3.get("status").unwrap(),
        "pending",
        "Main job should no longer be pending after dependency is complete"
    );
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
    assert_ne!(
        main_job_after.get("status").unwrap(),
        "pending",
        "Main job should not be pending after dependency is complete"
    );
}
