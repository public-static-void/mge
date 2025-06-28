use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{AiEventReactionSystem, assign_jobs, setup_ai_event_subscriptions};
use serde_json::json;

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

#[test]
fn test_event_driven_ai_job_enqueue() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();

    // Add an agent
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "production": 5.0 },
                "preferences": {},
                "state": "idle",
                "job_queue": []
            }),
        )
        .unwrap();

    // Add a production job for "wood"
    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "production",
                "state": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "wood", "amount": 10 } ],
                "category": "production"
            }),
        )
        .unwrap();

    // Setup AI event subscription
    setup_ai_event_subscriptions(&mut world);

    // Simulate a resource shortage event
    world
        .ai_event_intents
        .push_back(json!({ "kind": "wood", "amount": 0 }));

    // Run the AI event reaction system
    let mut system = AiEventReactionSystem;
    use engine_core::ecs::system::System;
    system.run(&mut world, None);

    // Agent's job queue should now contain the production job for wood
    let agent = world.get_component(agent_id, "Agent").unwrap();
    let queue = agent.get("job_queue").unwrap().as_array().unwrap();
    assert!(queue.iter().any(|v| v.as_u64() == Some(job_id as u64)));

    // Now assign jobs
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(agent_id, "Agent").unwrap();
    assert_eq!(agent["current_job"], job_id);
    assert_eq!(agent["state"], "working");
}

#[test]
fn test_event_intent_queue_handles_multiple_events() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();

    // Add an agent
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "production": 5.0 },
                "preferences": {},
                "state": "idle",
                "job_queue": []
            }),
        )
        .unwrap();

    // Add two production jobs
    let job_id1 = world.spawn_entity();
    world
        .set_component(
            job_id1,
            "Job",
            json!({
                "id": job_id1,
                "job_type": "production",
                "state": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "stone", "amount": 5 } ],
                "category": "production"
            }),
        )
        .unwrap();

    let job_id2 = world.spawn_entity();
    world
        .set_component(
            job_id2,
            "Job",
            json!({
                "id": job_id2,
                "job_type": "production",
                "state": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "wood", "amount": 5 } ],
                "category": "production"
            }),
        )
        .unwrap();

    // Setup AI event subscription
    setup_ai_event_subscriptions(&mut world);

    // Simulate two resource shortage events
    world
        .ai_event_intents
        .push_back(json!({ "kind": "wood", "amount": 0 }));
    world
        .ai_event_intents
        .push_back(json!({"kind": "stone", "amount": 0 }));

    // Run the AI event reaction system
    let mut system = AiEventReactionSystem;
    use engine_core::ecs::system::System;
    system.run(&mut world, None);

    // Agent's job queue should now contain both jobs
    let agent = world.get_component(agent_id, "Agent").unwrap();
    let queue = agent.get("job_queue").unwrap().as_array().unwrap();
    assert!(queue.iter().any(|v| v.as_u64() == Some(job_id1 as u64)));
    assert!(queue.iter().any(|v| v.as_u64() == Some(job_id2 as u64)));
}
