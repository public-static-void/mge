#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::equipment_logic::EquipmentLogicSystem;
use serde_json::json;

#[test]
fn test_register_equipment_schema_and_equip_items() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();

    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "head": null,
            "torso": null,
            "left_hand": null,
            "right_hand": null
        }
    });
    assert!(
        world
            .set_component(eid, "Equipment", equipment.clone())
            .is_ok()
    );

    // Equip a helmet
    let mut updated = equipment.clone();
    updated["slots"]["head"] = json!("iron_helmet");
    assert!(
        world
            .set_component(eid, "Equipment", updated.clone())
            .is_ok()
    );

    // Unequip the helmet
    updated["slots"]["head"] = json!(null);
    assert!(
        world
            .set_component(eid, "Equipment", updated.clone())
            .is_ok()
    );
}

#[test]
fn test_cannot_equip_incompatible_item() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();

    world.register_system(EquipmentLogicSystem);

    // Create an item that can only go in "head"
    let helmet_id = world.spawn_entity();
    let helmet = json!({
        "id": "iron_helmet",
        "name": "Iron Helmet",
        "slot": "head"
    });
    world.set_component(helmet_id, "Item", helmet).unwrap();

    // Create equipment with empty slots
    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "head": null,
            "left_hand": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    // Try to equip helmet in left_hand (should succeed at schema level)
    let mut updated = equipment.clone();
    updated["slots"]["left_hand"] = json!("iron_helmet");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(
        result.is_ok(),
        "set_component should succeed for schema-valid data"
    );

    // Run the logic system to enforce slot compatibility
    world.run_system("EquipmentLogicSystem", None).unwrap();

    // The system should have auto-unequipped the incompatible item
    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    assert!(
        equipment_after["slots"]["left_hand"].is_null(),
        "Incompatible item should be auto-unequipped"
    );
}

#[test]
fn test_equipping_two_handed_weapon_blocks_both_hands() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);

    // Create a two-handed weapon item
    let two_handed_weapon_id = world.spawn_entity();
    let two_handed_weapon = json!({
        "id": "greatsword",
        "name": "Greatsword",
        "slot": "left_hand",
        "two_handed": true
    });
    world
        .set_component(two_handed_weapon_id, "Item", two_handed_weapon)
        .unwrap();

    // Create equipment with empty hands
    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "left_hand": null,
            "right_hand": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    // Equip the two-handed weapon in left_hand
    let mut updated = equipment.clone();
    updated["slots"]["left_hand"] = json!("greatsword");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(result.is_ok());

    world.run_system("EquipmentLogicSystem", None).unwrap();

    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    assert_eq!(equipment_after["slots"]["left_hand"], "greatsword");
    assert_eq!(equipment_after["slots"]["right_hand"], "greatsword");
}

#[test]
fn test_cannot_equip_two_handed_weapon_if_other_hand_occupied() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);

    // Create a one-handed weapon item
    let sword_id = world.spawn_entity();
    let sword = json!({
        "id": "sword",
        "name": "Sword",
        "slot": "right_hand",
        "two_handed": false
    });
    world.set_component(sword_id, "Item", sword).unwrap();

    // Create a two-handed weapon item
    let greatsword_id = world.spawn_entity();
    let greatsword = json!({
        "id": "greatsword",
        "name": "Greatsword",
        "slot": "left_hand",
        "two_handed": true
    });
    world
        .set_component(greatsword_id, "Item", greatsword)
        .unwrap();

    // Create equipment with sword equipped in right_hand
    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "left_hand": null,
            "right_hand": "sword"
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    // Try to equip two-handed weapon in left_hand
    let mut updated = equipment.clone();
    updated["slots"]["left_hand"] = json!("greatsword");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(result.is_ok());

    world.run_system("EquipmentLogicSystem", None).unwrap();

    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    // The two-handed weapon should not be equipped because right_hand is occupied
    assert!(equipment_after["slots"]["left_hand"].is_null());
    assert_eq!(equipment_after["slots"]["right_hand"], "sword");
}

#[test]
fn test_cannot_equip_item_with_unmet_stat_requirement() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);

    // Create an item with strength requirement 10
    let heavy_armor_id = world.spawn_entity();
    let heavy_armor = json!({
        "id": "heavy_armor",
        "name": "Heavy Armor",
        "slot": "torso",
        "requirements": {
            "strength": 10
        }
    });
    world
        .set_component(heavy_armor_id, "Item", heavy_armor)
        .unwrap();

    // Create equipment with empty torso slot
    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "torso": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    // Create a stats component with strength 5 (insufficient)
    let stats = json!({
        "strength": 5
    });
    world.set_component(eid, "Stats", stats).unwrap();

    // Try to equip heavy armor
    let mut updated = equipment.clone();
    updated["slots"]["torso"] = json!("heavy_armor");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(result.is_ok());

    world.run_system("EquipmentLogicSystem", None).unwrap();

    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    // The heavy armor should be auto-unequipped due to unmet strength requirement
    assert!(equipment_after["slots"]["torso"].is_null());
}

#[test]
fn test_can_equip_item_with_met_stat_requirement() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);

    // Create an item with strength requirement 5
    let light_armor_id = world.spawn_entity();
    let light_armor = json!({
        "id": "light_armor",
        "name": "Light Armor",
        "slot": "torso",
        "requirements": {
            "strength": 5
        }
    });
    world
        .set_component(light_armor_id, "Item", light_armor)
        .unwrap();

    // Create equipment with empty torso slot
    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "torso": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    // Create a stats component with strength 10 (sufficient)
    let stats = json!({
        "strength": 10
    });
    world.set_component(eid, "Stats", stats).unwrap();

    // Equip light armor
    let mut updated = equipment.clone();
    updated["slots"]["torso"] = json!("light_armor");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(result.is_ok());

    world.run_system("EquipmentLogicSystem", None).unwrap();

    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    // The light armor should remain equipped
    assert_eq!(equipment_after["slots"]["torso"], "light_armor");
}
