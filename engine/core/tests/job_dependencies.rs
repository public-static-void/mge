#[path = "helpers/world.rs"]
mod world_helper;

#[path = "helpers/resource.rs"]
mod resource_helper;
use resource_helper::ResourceTestHelpers;

#[path = "helpers/agent.rs"]
mod agent_helper;
use agent_helper::AgentTestHelpers;

#[path = "helpers/test_tick.rs"]
mod test_tick_helper;
use test_tick_helper::run_until;

use engine_core::ecs::system::System;
use engine_core::systems::job::{JobBoard, JobSystem};
use serde_json::json;

const MAX_TICKS: usize = 16;

/// Tests that a job with an unfinished dependency remains pending.
#[test]
fn test_job_with_unfinished_dependency_remains_pending() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "id": dep_eid,
                "job_type": "dig",
                "state": "pending",
                "category": "mining"
            }),
        )
        .unwrap();

    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "id": main_eid,
                "job_type": "build",
                "state": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction"
            }),
        )
        .unwrap();

    // DO NOT SPAWN AGENT YET!

    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    // Run for MAX_TICKS with no agent, nothing should progress
    for tick in 0..MAX_TICKS {
        job_board.update(&world, tick as u64, &[]);
        engine_core::systems::job::ai::logic::assign_jobs(
            &mut world,
            &mut job_board,
            tick as u64,
            &[],
        );
        job_system.run(&mut world, None);
    }
    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(main_job_after.get("state").unwrap(), "pending");

    // Now spawn agent and manually complete the dependency
    world.spawn_idle_agent();
    let mut dep_job = world.get_component(dep_eid, "Job").unwrap().clone();
    dep_job["state"] = json!("complete");
    dep_job["id"] = json!(dep_eid);
    world.set_component(dep_eid, "Job", dep_job).unwrap();

    // Now main job should progress
    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main_eid, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job_after2 = world.get_component(main_eid, "Job").unwrap();
    assert_ne!(main_job_after2.get("state").unwrap(), "pending");
}

/// Tests that a job with a completed dependency can start.
#[test]
fn test_job_with_completed_dependency_can_start() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "id": dep_eid,
                "job_type": "dig",
                "state": "complete",
                "category": "mining"
            }),
        )
        .unwrap();

    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "id": main_eid,
                "job_type": "build",
                "state": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction"
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main_eid, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_ne!(main_job_after.get("state").unwrap(), "pending");
}

/// Tests that a job with AND dependencies can start when all are complete.
#[test]
fn test_job_with_and_dependencies() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep1 = world.spawn_entity();
    let dep2 = world.spawn_entity();
    let main = world.spawn_entity();

    world
        .set_component(
            dep1,
            "Job",
            json!({"id": dep1, "state":"complete","job_type":"a","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            dep2,
            "Job",
            json!({"id": dep2, "state":"complete","job_type":"b","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            main,
            "Job",
            json!({
                "id": main,
                "job_type":"main",
                "state":"pending",
                "category":"test",
                "dependencies": { "all_of": [dep1.to_string(), dep2.to_string()] }
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}

/// Tests that a job with OR dependencies can start when any is complete.
#[test]
fn test_job_with_or_dependencies() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep1 = world.spawn_entity();
    let dep2 = world.spawn_entity();
    let main = world.spawn_entity();

    world
        .set_component(
            dep1,
            "Job",
            json!({"id": dep1, "state":"failed","job_type":"a","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            dep2,
            "Job",
            json!({"id": dep2, "state":"complete","job_type":"b","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            main,
            "Job",
            json!({
                "id": main,
                "job_type":"main",
                "state":"pending",
                "category":"test",
                "dependencies": { "any_of": [dep1.to_string(), dep2.to_string()] }
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}

/// Tests that a job with a NOT dependency does not start if the dependency is failed,
/// and does start after the dependency is despawned.
#[test]
fn test_job_with_not_dependency() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep1 = world.spawn_entity();
    let main = world.spawn_entity();

    world
        .set_component(
            dep1,
            "Job",
            json!({"id": dep1, "state":"failed","job_type":"a","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            main,
            "Job",
            json!({
                "id": main,
                "job_type":"main",
                "state":"pending",
                "category":"test",
                "dependencies": { "not": [dep1.to_string()] }
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_eq!(main_job.get("state").unwrap(), "pending");

    // Now remove dep1 (simulate dep1 never existed)
    world.despawn_entity(dep1);

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}

/// Tests that a job with a world state dependency remains pending until the resource is available.
#[test]
fn test_job_with_world_state_dependency() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let stockpile = world.spawn_entity();
    world
        .set_component(stockpile, "Stockpile", json!({"resources": {}}))
        .unwrap();

    let main = world.spawn_entity();

    world.set_global_resource("water", 5.0);

    world
        .set_component(
            main,
            "Job",
            json!({
                "id": main,
                "job_type":"main",
                "state":"pending",
                "category":"test",
                "dependencies": [
                    { "world_state": { "resource": "water", "gte": 10.0 } }
                ]
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_eq!(main_job.get("state").unwrap(), "pending");

    world.set_global_resource("water", 10.0);

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}

/// Tests that a job with an entity state dependency remains pending until the condition is met.
#[test]
fn test_job_with_entity_state_dependency() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let entity = world.spawn_entity();
    let main = world.spawn_entity();

    world
        .set_component(entity, "Health", json!({"current": 5.0, "max": 10.0}))
        .unwrap();

    world.set_component(main, "Job", json!({
        "id": main,
        "job_type":"main",
        "state":"pending",
        "category":"test",
        "dependencies": [
            { "entity_state": { "entity": entity, "component": "Health", "field": "current", "gte": 10 } }
        ]
    })).unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_eq!(main_job.get("state").unwrap(), "pending");

    world
        .set_component(entity, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}
