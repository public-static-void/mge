#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::{JobLogicKind, JobSystem, JobTypeData};
use serde_json::json;

#[test]
fn test_partial_effect_rollback_on_cancel_and_interruption() {
    let mut world = world_helper::make_test_world();

    // Register minimal schemas for test components in "colony" mode
    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(
            &json!({
                "title": "Step",
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
                "title": "Agent",
                "type": "object",
                "properties": {
                    "entity_id": { "type": "integer" },
                    "state": { "type": "string" }
                },
                "required": ["entity_id", "state"],
                "modes": ["colony"]
            })
            .to_string(),
        )
        .unwrap();

    // Register effect handlers and their undo handlers with debug prints
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("Step1", |world, eid, _effect| {
            if let Err(_e) = world.set_component(eid, "Step", json!({"value": 1})) {}
        });
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("Step2", |world, eid, _effect| {
            if let Err(_e) = world.set_component(eid, "Step", json!({"value": 2})) {}
        });
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_undo_handler("UndoStep1", |world, eid, _effect| {
            if let Err(_e) = world.set_component(eid, "Step", json!({"value": 0})) {}
        });
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_undo_handler("UndoStep2", |world, eid, _effect| {
            if let Err(_e) = world.set_component(eid, "Step", json!({"value": 1})) {}
        });

    // Register the JobSystem
    world.systems.register_system(JobSystem::new());

    // Register job type with two effects
    let job_type_data = JobTypeData {
        name: "RollbackJob".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![
            serde_json::json!({ "action": "Step1" }),
            serde_json::json!({ "action": "Step2" }),
        ],
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
                "id": eid,
                "job_type": "RollbackJob",
                "state": "pending",
                "progress": 0.0,
                "category": "test"
            }),
        )
        .unwrap();

    // Run the job system once: should apply first effect only
    world.run_system("JobSystem", None).unwrap();
    let step = world.get_component(eid, "Step").unwrap();
    assert_eq!(step["value"], 1, "First effect should be applied");

    // Mark job as cancelled before second effect
    let mut job = world.get_component(eid, "Job").unwrap().clone();
    job["cancelled"] = json!(true);
    world.set_component(eid, "Job", job).unwrap();

    // Run the job system again: should rollback the first effect only
    world.run_system("JobSystem", None).unwrap();

    let step = world.get_component(eid, "Step").unwrap();
    assert_eq!(
        step["value"], 0,
        "First effect should be rolled back on cancel"
    );

    // Now test interruption (simulate agent death/unavailable)
    // Create a new job and agent
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "working"
            }),
        )
        .unwrap();

    let eid2 = world.spawn_entity();
    world
        .set_component(
            eid2,
            "Job",
            json!({
                "id": eid2,
                "job_type": "RollbackJob",
                "state": "pending",
                "progress": 0.0,
                "category": "test",
                "assigned_to": agent_id
            }),
        )
        .unwrap();

    // Run the job system once: first effect applied
    world.run_system("JobSystem", None).unwrap();

    let step = world.get_component(eid2, "Step").unwrap();
    assert_eq!(step["value"], 1, "First effect should be applied");

    // Simulate agent death/unavailability
    world.despawn_entity(agent_id);

    // Run the job system again: should detect missing agent and rollback
    world.run_system("JobSystem", None).unwrap();

    let step = world.get_component(eid2, "Step").unwrap();
    assert_eq!(
        step["value"], 0,
        "Effect should be rolled back on agent interruption"
    );
}
