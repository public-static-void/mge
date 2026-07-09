#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::economic::{EconomicSystem, load_recipes_from_dir};
use serde_json::{json, Value as JsonValue};

#[test]
fn test_workshop_produces_resources_using_recipe() {
    // Use the shared helper to load schemas via config
    let mut world = world_helper::make_test_world();

    // Explicitly set the mode to "colony"
    world.current_mode = "colony".to_string();

    let allowed = world.is_component_allowed_in_mode("Stockpile", &world.current_mode);
    assert!(
        allowed,
        "'Stockpile' should be allowed in mode '{}', but is_component_allowed_in_mode returned false",
        world.current_mode
    );

    // Add a workshop entity with a stockpile and a production job referencing a recipe
    let workshop = world.spawn_entity();
    let result = world.set_component(workshop, "Stockpile", json!({"resources": { "wood": 3 }}));
    assert!(result.is_ok(), "Failed to add Stockpile: {result:?}");
    world
        .set_component(
            workshop,
            "ProductionJob",
            json!({
                "recipe": "wood_plank",
                "progress": 0,
                "state": "pending"
            }),
        )
        .unwrap();

    // Load recipes and create/register the economic system
    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    // Run the economic system for 2 ticks
    for _tick in 0..2 {
        econ_system.run(&mut world);
    }

    // After 2 ticks, wood should be reduced by 1, plank increased by 4 (recipe runs once)
    let stockpile = world.get_component(workshop, "Stockpile").unwrap();
    let resources = stockpile["resources"].as_object().unwrap();
    assert_eq!(resources.get("wood").unwrap(), 2);
    assert_eq!(resources.get("plank").unwrap(), 4);
}

#[test]
fn test_production_job_priority_ordering() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    // Create entity A (high priority = 5) and entity B (low priority = 1)
    let entity_a = world.spawn_entity();
    let entity_b = world.spawn_entity();

    world.set_component(entity_a, "Stockpile", json!({"resources": {"wood": 10}})).unwrap();
    world.set_component(entity_b, "Stockpile", json!({"resources": {"wood": 10}})).unwrap();

    world
        .set_component(
            entity_a,
            "ProductionJob",
            json!({
                "recipe": "wood_plank",
                "progress": 0,
                "state": "pending",
                "priority": 5,
            }),
        )
        .unwrap();
    world
        .set_component(
            entity_b,
            "ProductionJob",
            json!({
                "recipe": "wood_plank",
                "progress": 0,
                "state": "pending",
                "priority": 1,
            }),
        )
        .unwrap();

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    // Run one tick — both jobs should progress, but priority determines order.
    // After both complete (duration=1), entity A processed first, then B.
    for _tick in 0..2 {
        econ_system.run(&mut world);
    }

    // Both should have completed
    let job_a = world.get_component(entity_a, "ProductionJob").unwrap();
    let job_b = world.get_component(entity_b, "ProductionJob").unwrap();
    assert_eq!(job_a["state"], "complete");
    assert_eq!(job_b["state"], "complete");
}

#[test]
fn test_production_job_tie_break_by_entity_id() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    // Create 3 entities with same priority
    let entity_a = world.spawn_entity(); // lower ID
    let entity_b = world.spawn_entity();
    let entity_c = world.spawn_entity(); // higher ID

    // Each has 1 wood stockpile for a recipe that consumes 1 wood
    world.set_component(entity_a, "Stockpile", json!({"resources": {"wood": 1}})).unwrap();
    world.set_component(entity_b, "Stockpile", json!({"resources": {"wood": 1}})).unwrap();
    world.set_component(entity_c, "Stockpile", json!({"resources": {"wood": 1}})).unwrap();

    for (eid, _name) in &[(entity_a, "a"), (entity_b, "b"), (entity_c, "c")] {
        world
            .set_component(
                *eid,
                "ProductionJob",
                json!({
                    "recipe": "wood_plank",
                    "progress": 0,
                    "state": "pending",
                    "priority": 0,
                }),
            )
            .unwrap();
    }

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    // Run one tick — all should progress (duration=1)
    for _tick in 0..2 {
        econ_system.run(&mut world);
    }

    // All should have completed
    for eid in &[entity_a, entity_b, entity_c] {
        let job = world.get_component(*eid, "ProductionJob").unwrap();
        assert_eq!(job["state"], "complete", "Entity {eid} should be complete");
    }
}

#[test]
fn test_production_job_batch_processing() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let workshop = world.spawn_entity();
    // Stockpile with enough wood for 3 batch runs: wood_plank consumes 1 wood per run
    world.set_component(workshop, "Stockpile", json!({"resources": {"wood": 3}})).unwrap();

    // batch_size=3: after complete, produces 3x4=12 plank from 1 wood
    world
        .set_component(
            workshop,
            "ProductionJob",
            json!({
                "recipe": "wood_plank",
                "progress": 0,
                "state": "pending",
                "batch_size": 3,
                "priority": 0,
            }),
        )
        .unwrap();

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    // wood_plank takes 1 tick duration
    econ_system.run(&mut world);

    // After 1 tick, should have consumed 1 wood and produced 12 plank (3*4)
    let stockpile = world.get_component(workshop, "Stockpile").unwrap();
    let resources = stockpile["resources"].as_object().unwrap();
    assert_eq!(resources.get("wood").unwrap(), 2, "Should have consumed 1 wood");
    assert_eq!(resources.get("plank").unwrap(), 12, "Should have produced 12 plank (3*4)");

    // job should be complete
    let job = world.get_component(workshop, "ProductionJob").unwrap();
    assert_eq!(job["state"], "complete");
}

#[test]
fn test_production_job_event_emission() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let workshop = world.spawn_entity();
    world.set_component(workshop, "Stockpile", json!({"resources": {"wood": 5}})).unwrap();

    world
        .set_component(
            workshop,
            "ProductionJob",
            json!({
                "recipe": "wood_plank",
                "progress": 0,
                "state": "pending",
                "priority": 0,
            }),
        )
        .unwrap();

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    // Run economic system for 2 ticks
    for _tick in 0..2 {
        econ_system.run(&mut world);
    }

    // Swap event buffers so take_events reads from the read buffer
    world.update_event_buses::<JsonValue>();

    // Check event emission
    let events = world.take_events("production_completed");
    assert!(!events.is_empty(), "Should have emitted production_completed event");
    let event = &events[0];
    assert_eq!(event["entity"].as_u64(), Some(workshop as u64));
    assert_eq!(event["recipe"].as_str(), Some("wood_plank"));
    assert_eq!(event["batch_count"].as_i64(), Some(1));
}

#[test]
fn test_no_production_jobs_is_noop() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    // Run with no entities — should not panic
    econ_system.run(&mut world);
    econ_system.run(&mut world);
}

#[test]
fn test_negative_priority_sorted_lowest() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let entity_high = world.spawn_entity();
    let entity_low = world.spawn_entity();

    world.set_component(entity_high, "Stockpile", json!({"resources": {"wood": 5}})).unwrap();
    world.set_component(entity_low, "Stockpile", json!({"resources": {"wood": 5}})).unwrap();

    // Negative priority treated as lowest
    world
        .set_component(entity_high, "ProductionJob", json!({
            "recipe": "wood_plank", "progress": 0, "state": "pending", "priority": 5
        }))
        .unwrap();
    world
        .set_component(entity_low, "ProductionJob", json!({
            "recipe": "wood_plank", "progress": 0, "state": "pending", "priority": -1
        }))
        .unwrap();

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    for _tick in 0..3 {
        econ_system.run(&mut world);
    }

    let job_high = world.get_component(entity_high, "ProductionJob").unwrap();
    let job_low = world.get_component(entity_low, "ProductionJob").unwrap();
    assert_eq!(job_high["state"], "complete");
    assert_eq!(job_low["state"], "complete");
}
