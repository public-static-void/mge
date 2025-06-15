#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::JobSystem;
use serde_json::json;

#[test]
fn test_job_with_failed_dependency_fails() {
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

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
    assert_eq!(
        main_job_after.get("status").unwrap(),
        "failed",
        "Main job should fail when dependency fails"
    );
}

#[test]
fn test_job_with_cancelled_dependency_cancels() {
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

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
    assert_eq!(
        main_job_after.get("status").unwrap(),
        "cancelled",
        "Main job should cancel when dependency is cancelled"
    );
}

#[test]
fn test_job_spawns_child_on_dependency_failure() {
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

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

    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(
        main_job_after.get("status").unwrap(),
        "failed",
        "Main job should fail when dependency fails"
    );
    let children = main_job_after
        .get("children")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(children.len(), 1, "Main job should have one child job");
    assert_eq!(
        children[0].get("job_type").unwrap(),
        "notify",
        "Child job should be of type 'notify'"
    );
    assert_eq!(
        children[0].get("status").unwrap(),
        "pending",
        "Child job should be pending"
    );
    assert_eq!(
        children[0].get("category").unwrap(),
        "notification",
        "Child job should be in the 'notification' category"
    );
}
