#[path = "helpers/agent.rs"]
mod agent_helper;
#[path = "helpers/world.rs"]
mod world_helper;

use agent_helper::AgentTestHelpers;
use engine_core::ecs::system::System;
use engine_core::systems::job::{JobBoard, JobLogicKind, JobTypeData, system::JobSystem};
use serde_json::json;

/// Verifies that job effects are processed when a job completes.
/// This test registers a job type with an effect that modifies terrain,
/// assigns the job to an entity, and ensures the effect is applied on completion.
#[test]
fn test_job_effects_are_processed_on_completion() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register an effect handler for "ModifyTerrain" that sets the Terrain type.
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("ModifyTerrain", |world, eid, effect| {
            if let Some(to) = effect.get("to").and_then(|v| v.as_str()) {
                world
                    .set_component(eid, "Terrain", json!({ "type": to, "kind": to }))
                    .unwrap();
            }
        });

    // Register a job type with an effect that modifies terrain.
    let job_type_data = JobTypeData {
        name: "DigTunnel".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    };
    world.job_types.register(
        job_type_data,
        JobLogicKind::Native(|_, _, _, job| job.clone()),
    );

    // Create an entity and assign the job.
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Job",
            json!({
                "id": eid, // <-- Make sure the job has an id!
                "job_type": "DigTunnel",
                "state": "pending",
                "category": "test",
                "progress": 0.0
            }),
        )
        .unwrap();

    // Spawn an idle agent so the job can be assigned and completed.
    world.spawn_idle_agent();

    // Run the job system enough times to complete the job and process effects.
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();
    for _ in 0..4 {
        job_board.update(&world, 0, &[]);
        engine_core::systems::job::ai::logic::assign_jobs(&mut world, &mut job_board, 0, &[]);
        job_system.run(&mut world, None);
    }

    // After job completion, the terrain type should be updated by the effect.
    let terrain = world.get_component(eid, "Terrain").unwrap();
    assert_eq!(
        terrain["type"], "tunnel",
        "Terrain type should be 'tunnel' after job completion"
    );
}
