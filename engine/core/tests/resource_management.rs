use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use std::sync::{Arc, Mutex};

#[test]
fn test_add_and_remove_resource() {
    // Load all schemas from the assets directory
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");

    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }

    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    // Add 10 wood
    let entity = world.spawn_entity();
    let res = world.set_component(
        entity,
        "Resource",
        serde_json::json!({
            "kind": "wood",
            "amount": 10
        }),
    );
    assert!(res.is_ok(), "Failed to add resource: {:?}", res);

    // Check resource amount
    let resource = world.get_component(entity, "Resource").unwrap();
    assert_eq!(resource["amount"], 10.0);

    // Remove 5 wood (simulate partial consumption)
    let res = world.modify_resource_amount(entity, "wood", -5.0);
    assert!(res.is_ok(), "Failed to remove resource: {:?}", res);

    let resource = world.get_component(entity, "Resource").unwrap();
    assert_eq!(resource["amount"], 5.0);

    // Removing more than available should error
    let res = world.modify_resource_amount(entity, "wood", -10.0);
    assert!(
        res.is_err(),
        "Should not be able to remove more than available"
    );
}
