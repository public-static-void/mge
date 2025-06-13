use engine_core::ecs::world::World;
use serde_json::json;

#[test]
fn job_effects_are_processed_on_completion() {
    let mut world = World::new(Default::default());

    // Register "Job" component for this test (allow in "colony" mode)
    let job_schema = serde_json::json!({
        "title": "Job",
        "type": "object",
        "properties": {
            "job_type": { "type": "string" },
            "status": { "type": "string" },
            "progress": { "type": "number" }
        },
        "required": ["job_type", "status", "progress"],
        "modes": ["colony"]
    });
    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(&job_schema.to_string())
        .unwrap();

    // Register "Terrain" component for this test (allow in "colony" mode)
    let terrain_schema = serde_json::json!({
        "title": "Terrain",
        "type": "object",
        "properties": {
            "type": { "type": "string" }
        },
        "required": ["type"],
        "modes": ["colony"]
    });
    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(&terrain_schema.to_string())
        .unwrap();

    // Register the JobSystem so it can be run by name
    world
        .systems
        .register_system(engine_core::systems::job::system::JobSystem::new());

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
                .set_component(eid, "Terrain", json!({ "type": to }))
                .unwrap();
        });

    // Add a job type with an effect
    let job_type_data = engine_core::systems::job::registry::JobTypeData {
        name: "DigTunnel".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    };
    world.job_types.register_data_job(job_type_data);

    // Create an entity and assign the job
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Job",
            json!({
                "job_type": "DigTunnel",
                "status": "pending",
                "progress": 0.0
            }),
        )
        .unwrap();

    // Run the job system enough times to complete the job
    for _ in 0..4 {
        world.run_system("JobSystem", None).unwrap();
    }

    let terrain = world.get_component(eid, "Terrain").unwrap();
    assert_eq!(terrain["type"], "tunnel");
}
