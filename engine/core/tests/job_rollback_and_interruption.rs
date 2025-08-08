#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::{JobLogicKind, JobSystem, JobTypeData};

#[test]
fn test_partial_effect_rollback_on_cancel_and_interruption() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register minimal schemas for test components in "colony" mode
    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(
            &serde_json::json!({
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
            &serde_json::json!({
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

    // Register effect handlers and their undo handlers
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("Step1", |world, eid, _effect| {
            let res = world.set_component(eid, "Step", serde_json::json!({"value": 1}));
            if res.is_err() {
                println!("Warning: Failed to set Step component in Step1 effect");
            }
        });
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("Step2", |world, eid, _effect| {
            let res = world.set_component(eid, "Step", serde_json::json!({"value": 2}));
            if res.is_err() {
                println!("Warning: Failed to set Step component in Step2 effect");
            }
        });
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_undo_handler("UndoStep1", |world, eid, _effect| {
            let res = world.set_component(eid, "Step", serde_json::json!({"value": 0}));
            if res.is_err() {
                println!("Warning: Failed to set Step component in UndoStep1 undo effect");
            }
        });
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_undo_handler("UndoStep2", |world, eid, _effect| {
            let res = world.set_component(eid, "Step", serde_json::json!({"value": 1}));
            if res.is_err() {
                println!("Warning: Failed to set Step component in UndoStep2 undo effect");
            }
        });

    // Register the JobSystem
    world.systems.register_system(JobSystem::new());

    // Register job type with two sequential effects
    let job_type_data = JobTypeData {
        name: "RollbackJob".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![
            serde_json::json!({ "action": "Step1" }),
            serde_json::json!({ "action": "Step2" }),
        ],
    };

    // Job handler closure
    let job_handler = |world: &mut engine_core::ecs::world::World,
                       _assigned_to: u32,
                       job_id: u32,
                       job: &serde_json::Value| {
        let mut job_map = job.as_object().cloned().unwrap_or_default();
        let job_type = job.get("job_type").and_then(|v| v.as_str()).unwrap_or("");

        engine_core::systems::job::system::effects::process_job_effects(
            world,
            job_id,
            job_type,
            &mut job_map,
            false,
        );
        serde_json::Value::Object(job_map)
    };

    world
        .job_types
        .register(job_type_data, JobLogicKind::Native(job_handler));
    world
        .job_handler_registry
        .lock()
        .unwrap()
        .register_handler("RollbackJob", job_handler);

    // Create an entity and assign the job with required_progress=2.0
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Job",
            serde_json::json!({
                "id": eid,
                "job_type": "RollbackJob",
                "state": "pending",
                "progress": 0.0,
                "required_progress": 2.0,
                "category": "test"
            }),
        )
        .unwrap();

    // Run the job system once: should apply first effect only (Step1)
    world.run_system("JobSystem", None).unwrap();

    // Check job component presence
    let _job_state = world
        .get_component(eid, "Job")
        .expect("Job component missing after first tick");

    // Check Step component set by first effect handler
    let step = world
        .get_component(eid, "Step")
        .expect("Step component missing after first tick");
    assert_eq!(step["value"], 1, "First effect should be applied");

    // Mark job as cancelled before second effect
    let mut job = world
        .get_component(eid, "Job")
        .expect("Job component missing for cancellation")
        .clone();
    job["state"] = serde_json::json!("cancelled");
    world.set_component(eid, "Job", job).unwrap();

    // Run the job system again: should rollback the first effect only
    world.run_system("JobSystem", None).unwrap();

    // Check Step component after rollback undo on cancellation
    let step = world
        .get_component(eid, "Step")
        .expect("Step component missing after cancellation rollback");
    assert_eq!(
        step["value"], 0,
        "First effect should be rolled back on cancel"
    );

    // Now test interruption (simulate agent death/unavailable)
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            serde_json::json!({
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
            serde_json::json!({
                "id": eid2,
                "job_type": "RollbackJob",
                "state": "pending",
                "progress": 0.0,
                "required_progress": 2.0,
                "category": "test",
                "assigned_to": agent_id
            }),
        )
        .unwrap();

    // Run the job system once: first effect applied (Step1)
    world.run_system("JobSystem", None).unwrap();

    let _job_state2 = world
        .get_component(eid2, "Job")
        .expect("Job component missing after first tick of second job");

    let step = world
        .get_component(eid2, "Step")
        .expect("Step component missing after first tick of second job");
    assert_eq!(step["value"], 1, "First effect should be applied");

    // Simulate agent death/unavailability
    world.despawn_entity(agent_id);

    // Confirm job entity still exists before rollback
    let job2_exists = world.entity_exists(eid2);
    assert!(job2_exists, "Job entity should still exist before rollback");

    // Run the job system again: should detect missing agent and rollback
    world.run_system("JobSystem", None).unwrap();

    // Check Step component after rollback on interruption
    let step_after_interrupt = world
        .get_component(eid2, "Step")
        .expect("Step component missing after interruption rollback");
    assert_eq!(
        step_after_interrupt["value"], 0,
        "Effect should be rolled back on agent interruption"
    );
}
