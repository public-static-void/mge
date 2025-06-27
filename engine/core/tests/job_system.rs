#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::JobSystem;
use serde_json::json;

#[test]
fn test_register_job_schema_and_assign_job_component() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "haul_resource",
        "target": 42,
        "state": "pending",
        "progress": 0.0,
        "category": "hauling"
    });
    assert!(world.set_component(eid, "Job", job_val.clone()).is_ok());

    let job = world.get_component(eid, "Job").unwrap();
    assert_eq!(job.get("job_type").unwrap(), "haul_resource");
    assert_eq!(job.get("state").unwrap(), "pending");
}

#[test]
fn test_query_entities_with_job_component() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();
    let eid = world.spawn_entity();

    let job_val = json!({
        "id": eid,
        "job_type": "build_structure",
        "state": "pending",
        "category": "construction"
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    let with_job = world.get_entities_with_component("Job");
    assert_eq!(with_job, vec![eid]);
}

#[test]
fn test_job_system_advances_progress_and_completes_job() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "test_job",
        "state": "pending",
        "progress": 0.0,
        "category": "testing"
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    world.register_system(JobSystem);

    for _ in 0..4 {
        world.run_system("JobSystem", None).unwrap();
    }

    let job = world.get_component(eid, "Job").unwrap();
    assert_eq!(job.get("state").unwrap(), "complete");
    assert!(job.get("progress").unwrap().as_f64().unwrap() >= 3.0);
}

#[test]
fn test_job_system_emits_event_on_completion() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "test_job",
        "state": "pending",
        "progress": 0.0,
        "category": "testing"
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    world.register_system(JobSystem);

    for _ in 0..6 {
        world.run_system("JobSystem", None).unwrap();
    }

    world.update_event_buses::<serde_json::Value>();

    let bus = world
        .get_event_bus::<serde_json::Value>("job_completed")
        .expect("event bus exists");
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();

    assert!(!events.is_empty(), "No job_completed events emitted");
    let found = events.iter().any(|event: &serde_json::Value| {
        event.get("entity").and_then(|v| v.as_u64()) == Some(eid as u64)
    });
    assert!(found, "No job_completed event for our entity");
}

#[test]
fn test_job_system_emits_event_on_failure() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "test_job",
        "state": "pending",
        "progress": 0.0,
        "should_fail": true,
        "category": "testing"
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    world.register_system(JobSystem);

    for _ in 0..2 {
        world.run_system("JobSystem", None).unwrap();
    }
    world.update_event_buses::<serde_json::Value>();

    let bus = world
        .get_event_bus::<serde_json::Value>("job_failed")
        .expect("event bus exists");
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();

    assert!(!events.is_empty(), "No job_failed events emitted");
    let found = events.iter().any(|event: &serde_json::Value| {
        event.get("entity").and_then(|v| v.as_u64()) == Some(eid as u64)
    });
    assert!(found, "No job_failed event for our entity");
}

#[test]
fn test_job_system_uses_custom_job_type_logic() {
    use engine_core::ecs::ComponentSchema;
    use engine_core::ecs::registry::ComponentRegistry;
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    let job_schema_json = std::fs::read_to_string("../assets/schemas/job.json").unwrap();
    let job_schema_value: Value = serde_json::from_str(&job_schema_json).unwrap();
    let name = job_schema_value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();
    let modes = job_schema_value
        .get("modes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let job_schema = ComponentSchema {
        name,
        schema: job_schema_value,
        modes,
    };
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    registry
        .lock()
        .unwrap()
        .register_external_schema(job_schema);

    let mut world = World::new(registry);
    world.current_mode = "colony".to_string();

    {
        let mut reg = world.job_handler_registry.lock().unwrap();
        reg.register_handler("fast_job", |_world, _agent_id, _job_id, job| {
            let mut job = job.clone();
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 10.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 10.0 {
                job["state"] = serde_json::json!("complete");
            } else {
                job["state"] = serde_json::json!("in_progress");
            }
            job
        });
    }

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "fast_job",
        "state": "pending",
        "progress": 0.0,
        "resource_requirements": [],
        "resource_outputs": [],
        "children": [],
        "dependencies": [],
        "category": "testing"
    });
    world.set_component(eid, "Job", job_val).unwrap();

    let mut job_system = JobSystem;
    for _ in 0..2 {
        System::run(&mut job_system, &mut world, None);
    }

    let job = world.get_component(eid, "Job").unwrap();

    assert_eq!(job.get("state").unwrap(), "complete");
    assert!(job.get("progress").unwrap().as_f64().unwrap() >= 10.0);
}

#[test]
fn test_job_assignment_is_recorded_and_queryable() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let worker_eid = world.spawn_entity();
    let job_eid = world.spawn_entity();
    let job_val = json!({
        "id": job_eid,
        "job_type": "dig_tunnel",
        "state": "pending",
        "assigned_to": worker_eid,
        "category": "mining"
    });
    world
        .set_component(job_eid, "Job", job_val.clone())
        .unwrap();

    let job = world.get_component(job_eid, "Job").unwrap();
    assert_eq!(
        job.get("assigned_to").unwrap().as_u64().unwrap(),
        worker_eid as u64,
        "Job should be assigned to worker"
    );
}
