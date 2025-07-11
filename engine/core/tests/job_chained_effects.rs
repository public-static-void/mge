#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::{JobLogicKind, JobSystem, JobTypeData};
use serde_json::json;

#[test]
fn test_job_chained_effects() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register minimal schemas for test components in "colony" mode
    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(
            &json!({
                "title": "FirstApplied",
                "type": "object",
                "properties": { "value": { "type": "integer" } },
                "required": ["value"],
                "modes": ["colony"]
            })
            .to_string(),
        )
        .unwrap();

    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(
            &json!({
                "title": "SecondApplied",
                "type": "object",
                "properties": { "value": { "type": "integer" } },
                "required": ["value"],
                "modes": ["colony"]
            })
            .to_string(),
        )
        .unwrap();

    // Register the JobSystem
    world.systems.register_system(JobSystem::new());

    // Register effect handlers
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("first", |world, eid, effect| {
            world
                .set_component(eid, "FirstApplied", json!({"value": effect["value"]}))
                .unwrap();
        });

    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("second", |world, eid, effect| {
            world
                .set_component(eid, "SecondApplied", json!({"value": effect["value"]}))
                .unwrap();
        });

    // Register job type with a chained effect (schema-driven)
    let job_type_data = JobTypeData {
        name: "ChainedJob".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![serde_json::json!({
            "action": "first",
            "value": 1,
            "effects": [
                { "action": "second", "value": 2 }
            ]
        })],
    };
    world.job_types.register(
        job_type_data,
        JobLogicKind::Native(|_, _, _, job| job.clone()),
    );

    // Create an entity and assign the job
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Job",
            json!({
                "job_type": "ChainedJob",
                "state": "pending",
                "progress": 0.0,
                "category": "test"
            }),
        )
        .unwrap();

    // Run the job system enough times to complete the job
    for _ in 0..4 {
        world.run_system("JobSystem", None).unwrap();
    }

    let first = world.get_component(eid, "FirstApplied").unwrap();
    assert_eq!(first["value"], 1, "First effect should be applied");

    let second = world.get_component(eid, "SecondApplied").unwrap();
    assert_eq!(
        second["value"], 2,
        "Second (chained) effect should be applied"
    );
}
