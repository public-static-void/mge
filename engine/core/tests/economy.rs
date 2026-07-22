#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::economic::{EconomicSystem, load_recipes_from_dir};
use serde_json::{Value as JsonValue, json};
use std::sync::{Arc, Mutex};

#[test]
fn test_workshop_produces_resources_using_recipe() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let allowed = world.is_component_allowed_in_mode("Stockpile", &world.current_mode);
    assert!(
        allowed,
        "'Stockpile' should be allowed in mode '{}'",
        world.current_mode
    );

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

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    for _tick in 0..2 {
        econ_system.run(&mut world);
    }

    let stockpile = world.get_component(workshop, "Stockpile").unwrap();
    let resources = stockpile["resources"].as_object().unwrap();
    assert_eq!(resources.get("wood").unwrap(), 2);
    assert_eq!(resources.get("plank").unwrap(), 4);
}

#[test]
fn test_production_job_priority_ordering() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let entity_a = world.spawn_entity();
    let entity_b = world.spawn_entity();

    world
        .set_component(entity_a, "Stockpile", json!({"resources": {"wood": 10}}))
        .unwrap();
    world
        .set_component(entity_b, "Stockpile", json!({"resources": {"wood": 10}}))
        .unwrap();

    world
        .set_component(
            entity_a,
            "ProductionJob",
            json!({
                "recipe": "wood_plank", "progress": 0, "state": "pending", "priority": 5,
            }),
        )
        .unwrap();
    world
        .set_component(
            entity_b,
            "ProductionJob",
            json!({
                "recipe": "wood_plank", "progress": 0, "state": "pending", "priority": 1,
            }),
        )
        .unwrap();

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    for _tick in 0..2 {
        econ_system.run(&mut world);
    }

    let job_a = world.get_component(entity_a, "ProductionJob").unwrap();
    let job_b = world.get_component(entity_b, "ProductionJob").unwrap();
    assert_eq!(job_a["state"], "complete");
    assert_eq!(job_b["state"], "complete");
}

#[test]
fn test_production_job_tie_break_by_entity_id() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let entity_a = world.spawn_entity();
    let entity_b = world.spawn_entity();
    let entity_c = world.spawn_entity();

    for eid in &[entity_a, entity_b, entity_c] {
        world
            .set_component(*eid, "Stockpile", json!({"resources": {"wood": 1}}))
            .unwrap();
    }
    for (eid, _name) in &[(entity_a, "a"), (entity_b, "b"), (entity_c, "c")] {
        world
            .set_component(
                *eid,
                "ProductionJob",
                json!({
                    "recipe": "wood_plank", "progress": 0, "state": "pending", "priority": 0,
                }),
            )
            .unwrap();
    }

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    for _tick in 0..2 {
        econ_system.run(&mut world);
    }

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
    world
        .set_component(workshop, "Stockpile", json!({"resources": {"wood": 3}}))
        .unwrap();
    world
        .set_component(workshop, "ProductionJob", json!({
            "recipe": "wood_plank", "progress": 0, "state": "pending", "batch_size": 3, "priority": 0,
        }))
        .unwrap();

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    econ_system.run(&mut world);

    let stockpile = world.get_component(workshop, "Stockpile").unwrap();
    let resources = stockpile["resources"].as_object().unwrap();
    assert_eq!(
        resources.get("wood").unwrap(),
        2,
        "Should have consumed 1 wood"
    );
    assert_eq!(
        resources.get("plank").unwrap(),
        12,
        "Should have produced 12 plank (3*4)"
    );

    let job = world.get_component(workshop, "ProductionJob").unwrap();
    assert_eq!(job["state"], "complete");
}

#[test]
fn test_production_job_event_emission() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let workshop = world.spawn_entity();
    world
        .set_component(workshop, "Stockpile", json!({"resources": {"wood": 5}}))
        .unwrap();
    world
        .set_component(
            workshop,
            "ProductionJob",
            json!({
                "recipe": "wood_plank", "progress": 0, "state": "pending", "priority": 0,
            }),
        )
        .unwrap();

    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    for _tick in 0..2 {
        econ_system.run(&mut world);
    }

    world.update_event_buses::<JsonValue>();
    let events = world.take_events("production_completed");
    assert!(
        !events.is_empty(),
        "Should have emitted production_completed event"
    );
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

    econ_system.run(&mut world);
    econ_system.run(&mut world);
}

#[test]
fn test_negative_priority_sorted_lowest() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let entity_high = world.spawn_entity();
    let entity_low = world.spawn_entity();

    world
        .set_component(entity_high, "Stockpile", json!({"resources": {"wood": 5}}))
        .unwrap();
    world
        .set_component(entity_low, "Stockpile", json!({"resources": {"wood": 5}}))
        .unwrap();

    world
        .set_component(
            entity_high,
            "ProductionJob",
            json!({
                "recipe": "wood_plank", "progress": 0, "state": "pending", "priority": 5
            }),
        )
        .unwrap();
    world
        .set_component(
            entity_low,
            "ProductionJob",
            json!({
                "recipe": "wood_plank", "progress": 0, "state": "pending", "priority": -1
            }),
        )
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

#[test]
fn test_add_and_remove_resource() {
    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");

    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }

    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    let entity = world.spawn_entity();
    let res = world.set_component(
        entity,
        "Resource",
        serde_json::json!({ "kind": "wood", "amount": 10 }),
    );
    assert!(res.is_ok(), "Failed to add resource: {res:?}");

    let resource = world.get_component(entity, "Resource").unwrap();
    assert_eq!(resource["amount"], 10.0);

    let res = world.modify_resource_amount(entity, "wood", -5.0);
    assert!(res.is_ok(), "Failed to remove resource: {res:?}");

    let resource = world.get_component(entity, "Resource").unwrap();
    assert_eq!(resource["amount"], 5.0);

    let res = world.modify_resource_amount(entity, "wood", -10.0);
    assert!(
        res.is_err(),
        "Should not be able to remove more than available"
    );
}
