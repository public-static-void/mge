use engine_core::ecs::ComponentSchema;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::JobSystem;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn can_register_job_schema_and_assign_job_component() {
    // 1. Setup registry and load Job schema
    let mut registry = ComponentRegistry::new();
    let job_schema_json = include_str!("../../assets/schemas/job.json");
    registry
        .register_external_schema_from_json(job_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    // 2. Create world
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    // 3. Spawn entity and assign Job component
    let eid = world.spawn_entity();
    let job_val = json!({
        "job_type": "haul_resource",
        "target": 42,
        "status": "pending",
        "progress": 0.0
    });
    assert!(world.set_component(eid, "Job", job_val.clone()).is_ok());

    // 4. Query entity for Job component
    let job = world.get_component(eid, "Job").unwrap();
    assert_eq!(job.get("job_type").unwrap(), "haul_resource");
    assert_eq!(job.get("status").unwrap(), "pending");
}

#[test]
fn can_query_entities_with_job_component() {
    let mut registry = ComponentRegistry::new();
    let job_schema_json = include_str!("../../assets/schemas/job.json");
    registry
        .register_external_schema_from_json(job_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();
    let eid = world.spawn_entity();

    let job_val = json!({
        "job_type": "build_structure",
        "status": "pending"
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    let with_job = world.get_entities_with_component("Job");
    assert_eq!(with_job, vec![eid]);
}

#[test]
fn job_system_advances_progress_and_completes_job() {
    use engine_core::systems::job::JobSystem;
    use serde_json::json;

    let mut registry = ComponentRegistry::new();
    let job_schema_json = include_str!("../../assets/schemas/job.json");
    registry
        .register_external_schema_from_json(job_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "test_job",
        "status": "pending",
        "progress": 0.0
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    world.register_system(JobSystem);

    // 1 tick: pending -> in_progress, 3 more: progress increments
    for _ in 0..4 {
        world.run_system("JobSystem", None).unwrap();
    }

    let job = world.get_component(eid, "Job").unwrap();
    assert_eq!(job.get("status").unwrap(), "complete");
    assert!(job.get("progress").unwrap().as_f64().unwrap() >= 3.0);
}

#[test]
fn job_system_emits_event_on_completion() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::job::JobSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    // Setup registry and world
    let mut registry = ComponentRegistry::new();
    let job_schema_json = include_str!("../../assets/schemas/job.json");
    registry
        .register_external_schema_from_json(job_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    // Spawn entity with a Job
    let eid = world.spawn_entity();
    let job_val = json!({
        "job_type": "test_job",
        "status": "pending",
        "progress": 0.0
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    // Register JobSystem
    world.register_system(JobSystem);

    // Run system enough times to complete the job
    for _ in 0..6 {
        world.run_system("JobSystem", None).unwrap();
    }

    world.update_event_buses::<serde_json::Value>();

    // Poll the event bus for "job_completed"
    let bus = world
        .get_event_bus::<serde_json::Value>("job_completed")
        .expect("event bus exists");
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();

    // There should be at least one event, and it should reference our entity
    assert!(!events.is_empty(), "No job_completed events emitted");
    let found = events.iter().any(|event: &serde_json::Value| {
        event.get("entity").and_then(|v| v.as_u64()) == Some(eid as u64)
    });
    assert!(found, "No job_completed event for our entity");
}

#[test]
fn job_system_emits_event_on_failure() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::job::JobSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let job_schema_json = include_str!("../../assets/schemas/job.json");
    registry
        .register_external_schema_from_json(job_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "test_job",
        "status": "pending",
        "progress": 0.0,
        "should_fail": true
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    world.register_system(JobSystem);

    // 1 tick: pending -> in_progress, 2nd: should_fail triggers "failed"
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
fn job_system_uses_custom_job_type_logic() {
    let job_schema_json = std::fs::read_to_string("../assets/schemas/job.json").unwrap();
    let job_schema_value: serde_json::Value = serde_json::from_str(&job_schema_json).unwrap();
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

    // Register the custom handler on the ACTUAL registry used by the world
    {
        let mut reg = world.job_handler_registry.lock().unwrap();
        reg.register_handler("fast_job", |_world, _agent_id, _job_id, job| {
            let mut job = job.clone();
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 10.0;
            println!("CUSTOM HANDLER: setting progress to {}", progress);
            job["progress"] = serde_json::json!(progress);
            if progress >= 10.0 {
                job["status"] = serde_json::json!("complete");
            } else {
                job["status"] = serde_json::json!("in_progress");
            }
            job
        });
        println!("TEST: Registered handler keys: {:?}", reg.keys());
    }

    // Insert a job with type "fast_job"
    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "fast_job",
        "status": "pending",
        "progress": 0.0,
        "resource_requirements": [],
        "resource_outputs": [],
        "children": [],
        "dependencies": []
    });
    world.set_component(eid, "Job", job_val).unwrap();

    // Register and run the system for a few ticks
    let mut job_system = JobSystem;
    for _ in 0..2 {
        System::run(&mut job_system, &mut world, None);
    }

    // Check the job after ticks
    let job = world.get_component(eid, "Job").unwrap();
    println!("job after ticks: {:?}", job);

    // The handler should have set progress to at least 10.0 and status to complete
    assert_eq!(job.get("status").unwrap(), "complete");
    assert!(job.get("progress").unwrap().as_f64().unwrap() >= 10.0);
}

#[test]
fn job_assignment_is_recorded_and_queryable() {
    use serde_json::json;

    let mut registry = ComponentRegistry::new();
    let job_schema_json = include_str!("../../assets/schemas/job.json");
    registry
        .register_external_schema_from_json(job_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    // Assign job to a worker entity
    let worker_eid = world.spawn_entity();
    let job_eid = world.spawn_entity();
    let job_val = json!({
        "job_type": "dig_tunnel",
        "status": "pending",
        "assigned_to": worker_eid
    });
    world
        .set_component(job_eid, "Job", job_val.clone())
        .unwrap();

    let job = world.get_component(job_eid, "Job").unwrap();
    assert_eq!(
        job.get("assigned_to").unwrap().as_u64().unwrap(),
        worker_eid as u64
    );
}
