#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::{JobLogicKind, JobSystem, JobTypeData};
use serde_json::json;

#[test]
fn test_job_effect_with_world_state_condition() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register the JobSystem
    world.systems.register_system(JobSystem::new());

    // Register a simple effect handler for "ModifyTerrain"
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("ModifyTerrain", |world, eid, effect| {
            let to = effect
                .get("to")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            world
                .set_component(eid, "Terrain", json!({ "kind": to }))
                .unwrap();
        });

    // Register job type with a conditional effect
    let job_type_data = JobTypeData {
        name: "ConditionalDig".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel",
            "condition": { "world_state": { "resource": "medkits", "gte": 1 } }
        })],
    };
    world.job_types.register(
        job_type_data,
        // ADVANCE progress and complete the job!
        JobLogicKind::Native(|_world, _eid, _actor, job| {
            let mut job = job.clone();
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 1.0 {
                job["state"] = serde_json::json!("complete");
            }
            job
        }),
    );

    // Create an agent with the correct specialization
    let agent = world.spawn_entity();
    world
        .set_component(
            agent,
            "Agent",
            json!({
                "entity_id": agent,
                "skills": {},
                "preferences": {},
                "state": "idle",
                "specializations": ["test"]
            }),
        )
        .unwrap();

    // Create the job and assign the agent
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Terrain", json!({ "kind": "rock" }))
        .unwrap();
    world
        .set_component(
            eid,
            "Job",
            json!({
                "job_type": "ConditionalDig",
                "state": "in_progress",
                "progress": 0.0,
                "category": "test",
                "assigned_to": agent
            }),
        )
        .unwrap();

    // Run the job system enough times to complete the job (no medkits yet)
    for _ in 0..4 {
        world.run_system("JobSystem", None).unwrap();
    }

    let terrain = world.get_component(eid, "Terrain").unwrap();
    assert_eq!(
        terrain["kind"], "rock",
        "Terrain kind should remain 'rock' when condition is not met"
    );

    // Now add medkits to the world
    world.set_global_resource_amount("medkits", 2.0);

    // Reset job to pending, assign agent, and run again
    world
        .set_component(
            eid,
            "Job",
            json!({
                "job_type": "ConditionalDig",
                "state": "in_progress",
                "progress": 0.0,
                "category": "test",
                "assigned_to": agent
            }),
        )
        .unwrap();

    for _ in 0..4 {
        world.run_system("JobSystem", None).unwrap();
    }

    let terrain = world.get_component(eid, "Terrain").unwrap();
    assert_eq!(
        terrain["kind"], "tunnel",
        "Terrain kind should become 'tunnel' when condition is met"
    );
}
