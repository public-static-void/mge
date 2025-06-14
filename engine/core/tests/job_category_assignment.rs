use std::path::Path;
use std::sync::{Arc, Mutex};

use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::systems::job::ai::assign_jobs;
use engine_core::systems::job_board::JobBoard;
use serde_json::json;

fn setup_world() -> World {
    let config =
        GameConfig::load_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"))
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
fn agent_prefers_job_matching_specialization_category() {
    let mut world = setup_world();

    // Agent 1 specializes in hauling
    let agent1_eid = world.spawn_entity();
    world
        .set_component(
            agent1_eid,
            "Agent",
            json!({
                "entity_id": agent1_eid,
                "specializations": ["hauling"],
                "skills": {},
                "preferences": {},
                "state": "idle"
            }),
        )
        .unwrap();

    // Agent 2 specializes in construction
    let agent2_eid = world.spawn_entity();
    world
        .set_component(
            agent2_eid,
            "Agent",
            json!({
                "entity_id": agent2_eid,
                "specializations": ["construction"],
                "skills": {},
                "preferences": {},
                "state": "idle"
            }),
        )
        .unwrap();

    // Job 1: hauling
    let job1_eid = world.spawn_entity();
    world
        .set_component(
            job1_eid,
            "Job",
            json!({
                "id": job1_eid,
                "job_type": "move_items",
                "category": "hauling",
                "status": "pending"
            }),
        )
        .unwrap();

    // Job 2: construction
    let job2_eid = world.spawn_entity();
    world
        .set_component(
            job2_eid,
            "Job",
            json!({
                "id": job2_eid,
                "job_type": "build_wall",
                "category": "construction",
                "status": "pending"
            }),
        )
        .unwrap();

    // Job 3: crafting (no agent specializes in this)
    let job3_eid = world.spawn_entity();
    world
        .set_component(
            job3_eid,
            "Job",
            json!({
                "id": job3_eid,
                "job_type": "make_tools",
                "category": "crafting",
                "status": "pending"
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    assign_jobs(&mut world, &mut job_board);

    // Agent 1 should get hauling job
    let agent1 = world.get_component(agent1_eid, "Agent").unwrap();
    let assigned_job1 = agent1.get("current_job").and_then(|v| v.as_u64()).unwrap() as u32;
    assert_eq!(assigned_job1, job1_eid);

    // Agent 2 should get construction job
    let agent2 = world.get_component(agent2_eid, "Agent").unwrap();
    let assigned_job2 = agent2.get("current_job").and_then(|v| v.as_u64()).unwrap() as u32;
    assert_eq!(assigned_job2, job2_eid);

    // Crafting job should remain unassigned
    let job3 = world.get_component(job3_eid, "Job").unwrap();
    assert!(job3.get("assigned_to").is_none());
}
