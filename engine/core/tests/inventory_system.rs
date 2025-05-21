use engine_core::systems::inventory::InventoryConstraintSystem;

#[test]
fn can_register_inventory_schema_and_manage_items() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let inventory_schema_json = include_str!("../../assets/schemas/inventory.json");
    registry
        .register_external_schema_from_json(inventory_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();
    world.register_system(InventoryConstraintSystem);

    let eid = world.spawn_entity();
    let inventory = json!({
        "slots": ["item_1", "item_2"],
        "max_slots": 5,
        "weight": 3.5,
        "max_weight": 10.0,
        "volume": 2.0,
        "max_volume": 5.0
    });
    assert!(
        world
            .set_component(eid, "Inventory", inventory.clone())
            .is_ok()
    );

    // Add an item
    let mut updated = inventory.clone();
    let mut slots = updated["slots"].as_array().unwrap().clone();
    slots.push(json!("item_3"));
    updated["slots"] = json!(slots);
    updated["weight"] = json!(4.0);
    updated["volume"] = json!(2.5);
    assert!(
        world
            .set_component(eid, "Inventory", updated.clone())
            .is_ok()
    );

    // Try to overfill slots
    let mut overfilled = updated.clone();
    let mut slots = overfilled["slots"].as_array().unwrap().clone();
    slots.push(json!("item_4"));
    slots.push(json!("item_5"));
    slots.push(json!("item_6")); // 6 items, but max_slots is 5
    overfilled["slots"] = json!(slots);
    let _ = world.set_component(eid, "Inventory", overfilled.clone());

    // Run the inventory constraint system
    world.run_system("InventoryConstraintSystem", None).unwrap();

    // Check that the encumbered flag is set
    let updated = world.get_component(eid, "Inventory").unwrap();
    assert_eq!(updated["encumbered"], true);
}

#[test]
fn inventory_constraint_system_sets_encumbered_status() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::inventory::InventoryConstraintSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let inventory_schema_json = include_str!("../../assets/schemas/inventory.json");
    registry
        .register_external_schema_from_json(inventory_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();
    world.register_system(InventoryConstraintSystem);

    let eid = world.spawn_entity();
    let inv = json!({
        "slots": ["item_1", "item_2"],
        "max_slots": 2,
        "weight": 3.5,
        "max_weight": 10.0,
        "volume": 2.0,
        "max_volume": 5.0
    });
    world.set_component(eid, "Inventory", inv).unwrap();

    // Add an item to overfill
    let mut inv = world.get_component(eid, "Inventory").unwrap().clone();
    let mut slots = inv["slots"].as_array().unwrap().clone();
    slots.push(json!("item_3"));
    inv["slots"] = json!(slots);
    world.set_component(eid, "Inventory", inv).unwrap();

    // Run the system
    world.run_system("InventoryConstraintSystem", None).unwrap();

    let updated = world.get_component(eid, "Inventory").unwrap();
    assert_eq!(updated["encumbered"], true);
}

#[test]
fn can_nest_inventories() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let inventory_schema_json = include_str!("../../assets/schemas/inventory.json");
    registry
        .register_external_schema_from_json(inventory_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();

    // Create a bag (container)
    let bag_id = world.spawn_entity();
    let bag_inventory = json!({
        "slots": [],
        "max_slots": 10,
        "weight": 0.5,
        "max_weight": 5.0,
        "volume": 0.5,
        "max_volume": 5.0
    });
    world
        .set_component(bag_id, "Inventory", bag_inventory)
        .unwrap();

    // Create a player inventory containing the bag
    let player_id = world.spawn_entity();
    let player_inventory = json!({
        "slots": [bag_id.to_string()],
        "max_slots": 5,
        "weight": 1.0,
        "max_weight": 10.0,
        "volume": 1.0,
        "max_volume": 10.0
    });
    world
        .set_component(player_id, "Inventory", player_inventory)
        .unwrap();

    // Optionally: Test for nested queries, constraints, etc.
}

#[test]
fn entity_is_encumbered_if_weight_exceeds_limit() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::inventory::InventoryConstraintSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let inventory_schema_json = include_str!("../../assets/schemas/inventory.json");
    registry
        .register_external_schema_from_json(inventory_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();
    world.register_system(InventoryConstraintSystem);

    // Create a nested inventory with high weight
    let heavy_bag = json!({
        "slots": [],
        "weight": 10.0,
        "volume": 0.5
    });
    let eid = world.spawn_entity();
    let inventory = json!({
        "slots": [heavy_bag],
        "weight": 5.0,
        "volume": 1.0,
        "max_weight": 12.0
    });
    world
        .set_component(eid, "Inventory", inventory.clone())
        .unwrap();

    world.run_system("InventoryConstraintSystem", None).unwrap();

    let inv_after = world.get_component(eid, "Inventory").unwrap();
    // Total weight = 5.0 + 10.0 = 15.0 > max_weight = 12.0
    assert_eq!(inv_after["encumbered"], true);
}
