use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::systems::job_board::{JobAssignmentResult, JobBoard};
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
fn test_job_board_tracks_unassigned_jobs() {
    let mut world = make_test_world_with_job_schema();
    let job1 = json!({"job_type": "mine", "status": "pending"});
    let job2 = json!({"job_type": "haul", "status": "pending", "assigned_to": 42});
    let eid1 = world.spawn_entity();
    let eid2 = world.spawn_entity();
    world.set_component(eid1, "Job", job1.clone()).unwrap();
    world.set_component(eid2, "Job", job2.clone()).unwrap();

    let mut board = JobBoard::default();
    board.update(&world);

    assert!(board.jobs.contains(&eid1));
    assert!(!board.jobs.contains(&eid2));
}

#[test]
fn test_job_assignment_claims_job() {
    let mut world = make_test_world_with_job_schema();
    let job = json!({"job_type": "build", "status": "pending"});
    let eid = world.spawn_entity();
    let actor_eid = world.spawn_entity();
    world.set_component(eid, "Job", job.clone()).unwrap();

    let mut board = JobBoard::default();
    board.update(&world);

    let result = board.claim_job(actor_eid, &mut world);
    assert_eq!(result, JobAssignmentResult::Assigned(eid));

    let assigned_job = world.get_component(eid, "Job").unwrap();
    assert_eq!(
        assigned_job.get("assigned_to").and_then(|v| v.as_u64()),
        Some(actor_eid as u64)
    );
}

#[test]
fn test_job_assignment_no_jobs_available() {
    let mut world = make_test_world_with_job_schema();
    let actor_eid = world.spawn_entity();

    let mut board = JobBoard::default();
    board.update(&world);

    let result = board.claim_job(actor_eid, &mut world);
    assert_eq!(result, JobAssignmentResult::NoJobsAvailable);
}
