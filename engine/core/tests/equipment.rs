#[path = "helpers/world.rs"]
mod world_helper;

#[path = "helpers/equipment.rs"]
mod equipment_helper;

use engine_core::systems::equipment_effect_aggregation::EquipmentEffectAggregationSystem;
use engine_core::systems::equipment_logic::EquipmentLogicSystem;
use engine_core::systems::stat_calculation::StatCalculationSystem;
use equipment_helper::{set_base_stats, setup_basic_equipment};
use serde_json::json;
use world_helper::make_test_world;

// === equipment_constraints tests ===

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

    let mut updated = equipment.clone();
    updated["slots"]["head"] = json!("iron_helmet");
    assert!(
        world
            .set_component(eid, "Equipment", updated.clone())
            .is_ok()
    );

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

    let helmet_id = world.spawn_entity();
    let helmet = json!({
        "id": "iron_helmet",
        "name": "Iron Helmet",
        "slot": "head"
    });
    world.set_component(helmet_id, "Item", helmet).unwrap();

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

    let mut updated = equipment.clone();
    updated["slots"]["left_hand"] = json!("iron_helmet");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(
        result.is_ok(),
        "set_component should succeed for schema-valid data"
    );

    world.run_system("EquipmentLogicSystem").unwrap();

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

    let mut updated = equipment.clone();
    updated["slots"]["left_hand"] = json!("greatsword");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(result.is_ok());

    world.run_system("EquipmentLogicSystem").unwrap();

    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    assert_eq!(equipment_after["slots"]["left_hand"], "greatsword");
    assert_eq!(equipment_after["slots"]["right_hand"], "greatsword");
}

#[test]
fn test_cannot_equip_two_handed_weapon_if_other_hand_occupied() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);

    let sword_id = world.spawn_entity();
    let sword = json!({
        "id": "sword",
        "name": "Sword",
        "slot": "right_hand",
        "two_handed": false
    });
    world.set_component(sword_id, "Item", sword).unwrap();

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

    let mut updated = equipment.clone();
    updated["slots"]["left_hand"] = json!("greatsword");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(result.is_ok());

    world.run_system("EquipmentLogicSystem").unwrap();

    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    assert!(equipment_after["slots"]["left_hand"].is_null());
    assert_eq!(equipment_after["slots"]["right_hand"], "sword");
}

#[test]
fn test_cannot_equip_item_with_unmet_stat_requirement() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);

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

    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "torso": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    let stats = json!({
        "strength": 5
    });
    world.set_component(eid, "Stats", stats).unwrap();

    let mut updated = equipment.clone();
    updated["slots"]["torso"] = json!("heavy_armor");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(result.is_ok());

    world.run_system("EquipmentLogicSystem").unwrap();

    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    assert!(equipment_after["slots"]["torso"].is_null());
}

#[test]
fn test_can_equip_item_with_met_stat_requirement() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);

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

    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "torso": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    let stats = json!({
        "strength": 10
    });
    world.set_component(eid, "Stats", stats).unwrap();

    let mut updated = equipment.clone();
    updated["slots"]["torso"] = json!("light_armor");
    let result = world.set_component(eid, "Equipment", updated.clone());
    assert!(result.is_ok());

    world.run_system("EquipmentLogicSystem").unwrap();

    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    assert_eq!(equipment_after["slots"]["torso"], "light_armor");
}

// === equipment_system tests ===

#[test]
fn test_equipping_item_applies_stat_bonuses() {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);
    world.register_system(EquipmentEffectAggregationSystem);
    world.register_system(StatCalculationSystem);

    let (_power_ring_id, eid) = setup_basic_equipment(&mut world);

    set_base_stats(&mut world, eid, 5.0, None);

    let equipment = world.get_component(eid, "Equipment").unwrap().clone();
    let mut updated = equipment.clone();
    updated["slots"]["finger"] = json!("power_ring");
    world.set_component(eid, "Equipment", updated).unwrap();

    world.run_system("EquipmentLogicSystem").unwrap();
    world
        .run_system("EquipmentEffectAggregationSystem")
        .unwrap();
    world.run_system("StatCalculationSystem").unwrap();

    let stats_after = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats_after["strength"].as_f64().unwrap(), 8.0);
}

#[test]
fn test_unequipping_item_removes_stat_bonuses_modular() {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentEffectAggregationSystem);
    world.register_system(StatCalculationSystem);

    let power_ring_id = world.spawn_entity();
    let power_ring = json!({
        "id": "power_ring",
        "name": "Power Ring",
        "slot": "finger",
        "effects": {
            "strength": 3
        }
    });
    world
        .set_component(power_ring_id, "Item", power_ring)
        .unwrap();

    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "finger": "power_ring"
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    set_base_stats(&mut world, eid, 5.0, None);

    world
        .run_system("EquipmentEffectAggregationSystem")
        .unwrap();
    world.run_system("StatCalculationSystem").unwrap();
    let strength_after = world.get_component(eid, "Stats").unwrap()["strength"]
        .as_f64()
        .unwrap();
    assert_eq!(strength_after, 8.0);

    let mut unequipped = equipment.clone();
    unequipped["slots"]["finger"] = json!(null);
    world.set_component(eid, "Equipment", unequipped).unwrap();

    world
        .run_system("EquipmentEffectAggregationSystem")
        .unwrap();
    world.run_system("StatCalculationSystem").unwrap();
    let strength_after_unequip = world.get_component(eid, "Stats").unwrap()["strength"]
        .as_f64()
        .unwrap();
    assert_eq!(strength_after_unequip, 5.0);
}

#[test]
fn test_equipping_item_applies_and_removes_effects_modular() {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentEffectAggregationSystem);
    world.register_system(StatCalculationSystem);

    let power_ring_id = world.spawn_entity();
    let power_ring = json!({
        "id": "power_ring",
        "name": "Power Ring",
        "slot": "finger",
        "effects": {
            "strength": 3,
            "dexterity": 2
        }
    });
    world
        .set_component(power_ring_id, "Item", power_ring)
        .unwrap();

    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "finger": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    set_base_stats(&mut world, eid, 5.0, Some(1.0));

    let mut updated = equipment.clone();
    updated["slots"]["finger"] = json!("power_ring");
    world.set_component(eid, "Equipment", updated).unwrap();

    world
        .run_system("EquipmentEffectAggregationSystem")
        .unwrap();
    world.run_system("StatCalculationSystem").unwrap();

    let stats_after = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats_after["strength"].as_f64().unwrap(), 8.0);
    assert_eq!(stats_after["dexterity"].as_f64().unwrap(), 3.0);

    let mut unequipped = equipment.clone();
    unequipped["slots"]["finger"] = json!(null);
    world.set_component(eid, "Equipment", unequipped).unwrap();

    world
        .run_system("EquipmentEffectAggregationSystem")
        .unwrap();
    world.run_system("StatCalculationSystem").unwrap();

    let stats_after_unequip = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats_after_unequip["strength"].as_f64().unwrap(), 5.0);
    assert_eq!(stats_after_unequip["dexterity"].as_f64().unwrap(), 1.0);
}
