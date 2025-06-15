#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::economic::{EconomicSystem, load_recipes_from_dir};
use serde_json::json;

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
    assert!(result.is_ok(), "Failed to add Stockpile: {:?}", result);
    world
        .set_component(
            workshop,
            "ProductionJob",
            json!({
                "recipe": "wood_plank",
                "progress": 0,
                "status": "pending"
            }),
        )
        .unwrap();

    // Print entities with each component after setup
    let entities_with_job = world.get_entities_with_component("ProductionJob");
    println!(
        "Entities with ProductionJob in test: {:?}",
        entities_with_job
    );
    let entities_with_stockpile = world.get_entities_with_component("Stockpile");
    println!(
        "Entities with Stockpile in test: {:?}",
        entities_with_stockpile
    );

    // Load recipes and create/register the economic system
    let recipe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/recipes";
    let recipes = load_recipes_from_dir(&recipe_dir);
    println!(
        "Loaded recipes: {:?}",
        recipes.iter().map(|r| &r.name).collect::<Vec<_>>()
    );
    let mut econ_system = EconomicSystem::with_recipes(recipes);

    // Run the economic system for 2 ticks
    for tick in 0..2 {
        println!("=== Tick {} ===", tick);
        econ_system.run(&mut world, None);
        println!(
            "Tick: job = {:?}, stockpile = {:?}",
            world.get_component(workshop, "ProductionJob"),
            world.get_component(workshop, "Stockpile")
        );
    }

    // After 2 ticks, wood should be reduced by 1, plank increased by 4 (recipe runs once)
    let stockpile = world.get_component(workshop, "Stockpile").unwrap();
    let resources = stockpile["resources"].as_object().unwrap();
    assert_eq!(resources.get("wood").unwrap(), 2);
    assert_eq!(resources.get("plank").unwrap(), 4);
}
