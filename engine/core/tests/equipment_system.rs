#[test]
fn equipping_item_applies_stat_bonuses() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::equipment_logic::EquipmentLogicSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let equipment_schema_json = include_str!("../../assets/schemas/equipment.json");
    let item_schema_json = include_str!("../../assets/schemas/item.json");
    let stats_schema_json = include_str!("../../assets/schemas/stats.json");
    registry
        .register_external_schema_from_json(equipment_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(item_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(stats_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();
    world.register_system(EquipmentLogicSystem);

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

    // Create a stats component with strength 5
    let stats = json!({
        "strength": 5
    });
    world.set_component(eid, "Stats", stats).unwrap();

    // Equip power ring
    let mut updated = equipment.clone();
    updated["slots"]["finger"] = json!("power_ring");
    world
        .set_component(eid, "Equipment", updated.clone())
        .unwrap();

    world.run_system("EquipmentLogicSystem", None).unwrap();

    let stats_after = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats_after["strength"].as_f64().unwrap(), 8.0);
}

#[test]
fn unequipping_item_removes_stat_bonuses_modular() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::equipment_effect_aggregation::EquipmentEffectAggregationSystem;
    use engine_core::systems::stat_calculation::StatCalculationSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let equipment_schema_json = include_str!("../../assets/schemas/equipment.json");
    let item_schema_json = include_str!("../../assets/schemas/item.json");
    let stats_schema_json = include_str!("../../assets/schemas/stats.json");
    let base_stats_schema_json = include_str!("../../assets/schemas/base_stats.json");
    let equipment_effects_schema_json = include_str!("../../assets/schemas/equipment_effects.json");
    registry
        .register_external_schema_from_json(equipment_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(item_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(stats_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(base_stats_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(equipment_effects_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
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
    let base_stats = json!({
        "strength": 5
    });
    world.set_component(eid, "BaseStats", base_stats).unwrap();

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
    world
        .set_component(eid, "Equipment", unequipped.clone())
        .unwrap();

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
fn equipping_item_applies_and_removes_effects_modular() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::equipment_effect_aggregation::EquipmentEffectAggregationSystem;
    use engine_core::systems::stat_calculation::StatCalculationSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let equipment_schema_json = include_str!("../../assets/schemas/equipment.json");
    let item_schema_json = include_str!("../../assets/schemas/item.json");
    let stats_schema_json = include_str!("../../assets/schemas/stats.json");
    let base_stats_schema_json = include_str!("../../assets/schemas/base_stats.json");
    let equipment_effects_schema_json = include_str!("../../assets/schemas/equipment_effects.json");
    registry
        .register_external_schema_from_json(equipment_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(item_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(stats_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(base_stats_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(equipment_effects_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
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
    let base_stats = json!({
        "strength": 5,
        "dexterity": 1
    });
    world.set_component(eid, "BaseStats", base_stats).unwrap();

    // Equip power ring
    let mut updated = equipment.clone();
    updated["slots"]["finger"] = json!("power_ring");
    world
        .set_component(eid, "Equipment", updated.clone())
        .unwrap();

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
    world
        .set_component(eid, "Equipment", unequipped.clone())
        .unwrap();

    world
        .run_system("EquipmentEffectAggregationSystem", None)
        .unwrap();
    world.run_system("StatCalculationSystem", None).unwrap();

    let stats_after_unequip = world.get_component(eid, "Stats").unwrap();
    assert_eq!(stats_after_unequip["strength"].as_f64().unwrap(), 5.0);
    assert_eq!(stats_after_unequip["dexterity"].as_f64().unwrap(), 1.0);
}
