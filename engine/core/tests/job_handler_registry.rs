use engine_core::ecs::world::World;
use engine_core::systems::job::builtin_handlers::register_builtin_job_handlers;
use engine_core::systems::job::job_handler_registry::JobHandlerRegistry;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_register_and_invoke_job_handler() {
    let registry = Arc::new(Mutex::new(JobHandlerRegistry::new()));

    let called = Arc::new(Mutex::new(false));
    let called_clone = called.clone();

    registry.lock().unwrap().register_handler(
        "test_job",
        move |_world, agent_id, job_id, _data| {
            assert_eq!(agent_id, 42);
            assert_eq!(job_id, 99);
            *called_clone.lock().unwrap() = true;
            serde_json::json!(null)
        },
    );

    let mut world = World::new(Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::default(),
    )));
    world.job_handler_registry = Arc::clone(&registry);

    // Clone the handler out to end the immutable borrow before mutably borrowing world
    let handler = world
        .job_handler_registry
        .lock()
        .unwrap()
        .get("test_job")
        .expect("Handler not found")
        .clone();
    handler(&world, 42, 99, &json!({}));

    assert!(*called.lock().unwrap());
}

#[test]
fn test_missing_job_handler() {
    let registry = Arc::new(Mutex::new(JobHandlerRegistry::new()));
    assert!(registry.lock().unwrap().get("nonexistent_job").is_none());
}

#[test]
fn test_data_driven_registration() {
    use engine_core::systems::job::registry::JobTypeRegistry;
    use std::fs;
    use tempfile::tempdir;

    // Create a temporary directory for job definitions
    let temp_dir = tempdir().expect("failed to create temp dir");
    let jobs_dir = temp_dir.path();

    // Create minimal job definition files for "production" and "haul"
    fs::write(
        jobs_dir.join("production.json"),
        r#"{"name":"production","type":"production"}"#,
    )
    .unwrap();
    fs::write(
        jobs_dir.join("haul.json"),
        r#"{"name":"haul","type":"haul"}"#,
    )
    .unwrap();

    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::default(),
    ));
    let mut world = World::new(registry);

    // Register dummy native logic for both job types
    let mut job_type_registry = JobTypeRegistry::default();
    job_type_registry.register_native(
        "production",
        Box::new(|_data, _progress| serde_json::json!(null)),
    );
    job_type_registry.register_native("haul", Box::new(|_data, _progress| serde_json::json!(null)));

    register_builtin_job_handlers(&mut world, &job_type_registry, jobs_dir);

    for job_type in &["production", "haul"] {
        assert!(
            world
                .job_handler_registry
                .lock()
                .unwrap()
                .get(job_type)
                .is_some(),
            "Handler for job type '{}' was not registered",
            job_type
        );
    }
}
