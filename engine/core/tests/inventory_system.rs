#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;
#[path = "helpers/inventory.rs"]
mod inventory_helper;
use inventory_helper::create_inventory;

use engine_core::systems::inventory::InventoryConstraintSystem;
use serde_json::json;

#[test]
fn test_can_register_inventory_schema_and_manage_items() {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(InventoryConstraintSystem);

    let eid = create_inventory(
        &mut world,
        vec![json!("item_1"), json!("item_2")],
        5,
        3.5,
        10.0,
        2.0,
        5.0,
    );

    // Add an item
    let mut updated = world.get_component(eid, "Inventory").unwrap().clone();
    let mut slots = updated["slots"].as_array().unwrap().clone();
    slots.push(json!("item_3"));
    updated["slots"] = json!(slots);
    updated["weight"] = json!(4.0);
    updated["volume"] = json!(2.5);
    world.set_component(eid, "Inventory", updated).unwrap();

    // Try to overfill slots
    let mut overfilled = world.get_component(eid, "Inventory").unwrap().clone();
    let mut slots = overfilled["slots"].as_array().unwrap().clone();
    slots.push(json!("item_4"));
    slots.push(json!("item_5"));
    slots.push(json!("item_6")); // 6 items, but max_slots is 5
    overfilled["slots"] = json!(slots);
    let _ = world.set_component(eid, "Inventory", overfilled);

    // Run the inventory constraint system
    world.run_system("InventoryConstraintSystem", None).unwrap();

    // Check that the encumbered flag is set
    let updated = world.get_component(eid, "Inventory").unwrap();
    assert_eq!(updated["encumbered"], true);
}

#[test]
fn test_inventory_constraint_system_sets_encumbered_status() {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(InventoryConstraintSystem);

    let eid = create_inventory(
        &mut world,
        vec![json!("item_1"), json!("item_2")],
        2,
        3.5,
        10.0,
        2.0,
        5.0,
    );

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
fn test_can_nest_inventories() {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();

    let bag_id = create_inventory(&mut world, vec![], 10, 0.5, 5.0, 0.5, 5.0);

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
}

#[test]
fn test_entity_is_encumbered_if_weight_exceeds_limit() {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(InventoryConstraintSystem);

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
    world.set_component(eid, "Inventory", inventory).unwrap();

    world.run_system("InventoryConstraintSystem", None).unwrap();

    let inv_after = world.get_component(eid, "Inventory").unwrap();
    assert_eq!(inv_after["encumbered"], true);
}
