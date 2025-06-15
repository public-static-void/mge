#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;
#[path = "helpers/equipment.rs"]
mod equipment_helper;
use equipment_helper::{set_base_stats, setup_basic_equipment};

use engine_core::systems::equipment_effect_aggregation::EquipmentEffectAggregationSystem;
use engine_core::systems::equipment_logic::EquipmentLogicSystem;
use engine_core::systems::stat_calculation::StatCalculationSystem;
use serde_json::json;

#[test]
fn test_equipping_item_applies_stat_bonuses() {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);

    let (_power_ring_id, eid) = setup_basic_equipment(&mut world);

    // Create a stats component with strength 5
    let stats = json!({
        "strength": 5
    });
    world.set_component(eid, "Stats", stats).unwrap();

    // Equip power ring
    let equipment = world.get_component(eid, "Equipment").unwrap().clone();
    let mut updated = equipment.clone();
    updated["slots"]["finger"] = json!("power_ring");
    world.set_component(eid, "Equipment", updated).unwrap();

    world.run_system("EquipmentLogicSystem", None).unwrap();

    let stats_after = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats_after["strength"].as_f64().unwrap(), 8.0);
}

#[test]
fn test_unequipping_item_removes_stat_bonuses_modular() {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentEffectAggregationSystem);
    world.register_system(StatCalculationSystem);

    // Create an item with effects: { "strength": 3 }
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

    // Create equipment with power_ring equipped in finger slot
    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "finger": "power_ring"
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    // Create a base stats component with strength 5
    set_base_stats(&mut world, eid, 5.0, None);

    // Run systems to apply effect
    world
        .run_system("EquipmentEffectAggregationSystem", None)
        .unwrap();
    world.run_system("StatCalculationSystem", None).unwrap();
    let strength_after = world.get_component(eid, "Stats").unwrap()["strength"]
        .as_f64()
        .unwrap();
    assert_eq!(strength_after, 8.0);

    // Unequip power ring
    let mut unequipped = equipment.clone();
    unequipped["slots"]["finger"] = json!(null);
    world.set_component(eid, "Equipment", unequipped).unwrap();

    world
        .run_system("EquipmentEffectAggregationSystem", None)
        .unwrap();
    world.run_system("StatCalculationSystem", None).unwrap();
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

    // Create an item with effects: { "strength": 3, "dexterity": 2 }
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

    // Create equipment with empty finger slot
    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "finger": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    // Create a base stats component with strength 5, dexterity 1
    set_base_stats(&mut world, eid, 5.0, Some(1.0));

    // Equip power ring
    let mut updated = equipment.clone();
    updated["slots"]["finger"] = json!("power_ring");
    world.set_component(eid, "Equipment", updated).unwrap();

    world
        .run_system("EquipmentEffectAggregationSystem", None)
        .unwrap();
    world.run_system("StatCalculationSystem", None).unwrap();

    let stats_after = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats_after["strength"].as_f64().unwrap(), 8.0);
    assert_eq!(stats_after["dexterity"].as_f64().unwrap(), 3.0);

    // Unequip power ring
    let mut unequipped = equipment.clone();
    unequipped["slots"]["finger"] = json!(null);
    world.set_component(eid, "Equipment", unequipped).unwrap();

    world
        .run_system("EquipmentEffectAggregationSystem", None)
        .unwrap();
    world.run_system("StatCalculationSystem", None).unwrap();

    let stats_after_unequip = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats_after_unequip["strength"].as_f64().unwrap(), 5.0);
    assert_eq!(stats_after_unequip["dexterity"].as_f64().unwrap(), 1.0);
}
