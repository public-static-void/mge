use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
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

    // Setup registry and world as before
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

    // Register and run the production JobSystem
    world.register_system(JobSystem::default());

    // Simulate ticks
    for _ in 0..5 {
        world.run_system("JobSystem", None).unwrap();
    }

    // Check that the job is now complete
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
    world.register_system(JobSystem::default());

    // Run system enough times to complete the job
    for _ in 0..6 {
        world.run_system("JobSystem", None).unwrap();
    }

    world.update_event_buses();

    // Poll the event bus for "job_completed"
    let bus = world
        .get_event_bus("job_completed")
        .expect("event bus exists");
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();

    // There should be at least one event, and it should reference our entity
    assert!(!events.is_empty(), "No job_completed events emitted");
    let found = events
        .iter()
        .any(|event| event.get("entity").and_then(|v| v.as_u64()) == Some(eid as u64));
    assert!(found, "No job_completed event for our entity");
}

#[test]
fn job_system_emits_event_on_failure() {
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

    // Spawn entity with a Job that should fail (simulate with a "should_fail": true field)
    let eid = world.spawn_entity();
    let job_val = json!({
        "job_type": "test_job",
        "status": "pending",
        "progress": 0.0,
        "should_fail": true
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    // Register JobSystem
    world.register_system(JobSystem::default());

    // Run system enough times to process the job
    for _ in 0..6 {
        world.run_system("JobSystem", None).unwrap();
    }
    world.update_event_buses();

    // Poll the event bus for "job_failed"
    let bus = world.get_event_bus("job_failed").expect("event bus exists");
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();

    // There should be at least one event, and it should reference our entity
    assert!(!events.is_empty(), "No job_failed events emitted");
    let found = events
        .iter()
        .any(|event| event.get("entity").and_then(|v| v.as_u64()) == Some(eid as u64));
    assert!(found, "No job_failed event for our entity");
}

#[test]
fn job_system_uses_custom_job_type_logic() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::job::{JobSystem, JobTypeRegistry};
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

    // Create a job type registry and register a custom job type
    let mut job_types = JobTypeRegistry::default();
    job_types.register_native(
        "fast_job",
        Box::new(|job: &serde_json::Value, progress: f64| {
            // Fast jobs complete in 1 tick
            let mut job = job.clone();
            let new_progress = progress + 10.0;
            job["progress"] = json!(new_progress);
            if new_progress >= 10.0 {
                job["status"] = json!("complete");
            }
            job
        }),
    );

    // Attach job_types to JobSystem (assume JobSystem takes a registry reference)
    let job_system = JobSystem::with_registry(job_types);

    // Spawn entity with a fast_job
    let eid = world.spawn_entity();
    let job_val = json!({
        "job_type": "fast_job",
        "status": "pending",
        "progress": 0.0
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    // Register and run the job system
    world.register_system(job_system);
    world.run_system("JobSystem", None).unwrap();
    world.update_event_buses();

    // Check that the job is now complete
    let job = world.get_component(eid, "Job").unwrap();
    assert_eq!(job.get("status").unwrap(), "complete");
    assert!(job.get("progress").unwrap().as_f64().unwrap() >= 10.0);
}

#[test]
fn hierarchical_job_completes_only_when_all_children_complete() {
    use engine_core::systems::job::JobSystem;
    use serde_json::json;

    // Setup registry and world
    let mut registry = ComponentRegistry::new();
    let job_schema_json = include_str!("../../assets/schemas/job.json");
    registry
        .register_external_schema_from_json(job_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    // Spawn parent entity with a Job that has two children
    let eid = world.spawn_entity();
    let child1 = json!({
        "job_type": "child_job",
        "status": "pending",
        "progress": 0.0
    });
    let child2 = json!({
        "job_type": "child_job",
        "status": "pending",
        "progress": 0.0
    });
    let parent_job = json!({
        "job_type": "parent_job",
        "status": "pending",
        "progress": 0.0,
        "children": [child1, child2]
    });
    world.set_component(eid, "Job", parent_job).unwrap();

    world.register_system(JobSystem::default());

    // Simulate ticks: children should complete first
    for _ in 0..4 {
        world.run_system("JobSystem", None).unwrap();
    }

    let job = world.get_component(eid, "Job").unwrap();
    // Parent should only be complete if both children are complete
    let children = job.get("children").unwrap().as_array().unwrap();
    assert!(
        children
            .iter()
            .all(|c| c.get("status").unwrap() == "complete")
    );
    assert_eq!(job.get("status").unwrap(), "complete");
}

#[test]
fn cancelling_parent_job_cancels_all_children() {
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

    // Parent job with two children
    let eid = world.spawn_entity();
    let child1 = json!({
        "job_type": "child_job",
        "status": "pending",
        "progress": 0.0
    });
    let child2 = json!({
        "job_type": "child_job",
        "status": "pending",
        "progress": 0.0
    });
    let parent_job = json!({
        "job_type": "parent_job",
        "status": "in_progress",
        "progress": 0.0,
        "cancelled": true,
        "children": [child1, child2]
    });
    world.set_component(eid, "Job", parent_job).unwrap();

    world.register_system(JobSystem::default());
    world.run_system("JobSystem", None).unwrap();

    let job = world.get_component(eid, "Job").unwrap();
    assert_eq!(job.get("status").unwrap(), "cancelled");
    let children = job.get("children").unwrap().as_array().unwrap();
    assert!(
        children
            .iter()
            .all(|c| c.get("status").unwrap() == "cancelled")
    );
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
