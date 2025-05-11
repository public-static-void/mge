use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::scripting::world::World;
use std::sync::Arc;

#[test]
fn test_add_and_remove_stockpile_resources() {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");

    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }

    let mut world = World::new(Arc::new(registry));
    world.current_mode = "colony".to_string();

    let entity = world.spawn_entity();

    // Add stockpile with wood and stone
    let res = world.set_component(
        entity,
        "Stockpile",
        serde_json::json!({
            "resources": { "wood": 10, "stone": 5 }
        }),
    );
    assert!(res.is_ok(), "Failed to add stockpile: {:?}", res);

    // Add 3 food
    let res = world.modify_stockpile_resource(entity, "food", 3.0);
    assert!(res.is_ok());

    // Remove 2 wood
    let res = world.modify_stockpile_resource(entity, "wood", -2.0);
    assert!(res.is_ok());

    // Check amounts
    let stockpile = world.get_component(entity, "Stockpile").unwrap();
    assert_eq!(stockpile["resources"]["wood"], 8.0);
    assert_eq!(stockpile["resources"]["stone"], 5.0);
    assert_eq!(stockpile["resources"]["food"], 3.0);

    // Removing more than available should error
    let res = world.modify_stockpile_resource(entity, "wood", -20.0);
    assert!(
        res.is_err(),
        "Should not be able to remove more than available"
    );
}
