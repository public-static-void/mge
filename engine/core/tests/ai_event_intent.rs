use engine_core::ecs::world::World;
use engine_core::systems::job::{
    ai_event_reaction_system::AiEventReactionSystem, assign_jobs, setup_ai_event_subscriptions,
};
use engine_core::systems::job_board::JobBoard;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_event_driven_ai_job_enqueue() {
    // Register Agent and Job schemas for colony mode
    let mut registry = engine_core::ecs::registry::ComponentRegistry::default();
    registry.register_external_schema(engine_core::ecs::schema::ComponentSchema {
        name: "Agent".to_string(),
        schema: serde_json::json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    registry.register_external_schema(engine_core::ecs::schema::ComponentSchema {
        name: "Job".to_string(),
        schema: serde_json::json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry);

    // Add an agent using set_component
    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "skills": { "production": 5.0 },
                "preferences": {},
                "state": "idle",
                "job_queue": []
            }),
        )
        .unwrap();
    world.entities.push(1);

    // Add a production job for "wood"
    world
        .set_component(
            100,
            "Job",
            json!({
                "id": 100,
                "job_type": "production",
                "status": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "wood", "amount": 10 } ]
            }),
        )
        .unwrap();
    world.entities.push(100);

    // Setup AI event subscription
    setup_ai_event_subscriptions(&mut world);

    // Simulate a resource shortage event
    world
        .ai_event_intents
        .push_back(json!({ "kind": "wood", "amount": 0 }));

    // Run the AI event reaction system
    let mut system = AiEventReactionSystem;
    use engine_core::ecs::system::System; // Bring trait into scope for .run()
    system.run(&mut world, None);

    // Agent's job queue should now contain the production job for wood
    let agent = world.get_component(1, "Agent").unwrap();
    let queue = agent.get("job_queue").unwrap().as_array().unwrap();
    assert!(queue.iter().any(|v| v.as_u64() == Some(100)));

    // Now assign jobs
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let agent = world.get_component(1, "Agent").unwrap();
    assert_eq!(agent["current_job"], 100);
    assert_eq!(agent["state"], "working");
}

#[test]
fn test_event_intent_queue_handles_multiple_events() {
    // Register Agent and Job schemas for colony mode
    let mut registry = engine_core::ecs::registry::ComponentRegistry::default();
    registry.register_external_schema(engine_core::ecs::schema::ComponentSchema {
        name: "Agent".to_string(),
        schema: serde_json::json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    registry.register_external_schema(engine_core::ecs::schema::ComponentSchema {
        name: "Job".to_string(),
        schema: serde_json::json!({ "type": "object" }),
        modes: vec!["colony".to_string()],
    });
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry);

    // Add an agent using set_component
    world
        .set_component(
            2,
            "Agent",
            json!({
                "entity_id": 2,
                "skills": { "production": 5.0 },
                "preferences": {},
                "state": "idle",
                "job_queue": []
            }),
        )
        .unwrap();
    world.entities.push(2);

    // Add two production jobs
    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "production",
                "status": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "stone", "amount": 5 } ]
            }),
        )
        .unwrap();
    world.entities.push(200);

    world
        .set_component(
            201,
            "Job",
            json!({
                "id": 201,
                "job_type": "production",
                "status": "pending",
                "priority": 1,
                "resource_outputs": [ { "kind": "wood", "amount": 5 } ]
            }),
        )
        .unwrap();
    world.entities.push(201);

    // Setup AI event subscription
    setup_ai_event_subscriptions(&mut world);

    // Simulate two resource shortage events
    world
        .ai_event_intents
        .push_back(json!({ "kind": "wood", "amount": 0 }));
    world
        .ai_event_intents
        .push_back(json!({ "kind": "stone", "amount": 0 }));

    // Run the AI event reaction system
    let mut system = AiEventReactionSystem;
    use engine_core::ecs::system::System;
    system.run(&mut world, None);

    // Agent's job queue should now contain both jobs
    let agent = world.get_component(2, "Agent").unwrap();
    let queue = agent.get("job_queue").unwrap().as_array().unwrap();
    assert!(queue.iter().any(|v| v.as_u64() == Some(200)));
    assert!(queue.iter().any(|v| v.as_u64() == Some(201)));
}
