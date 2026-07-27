#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::equipment_set::EquipmentSet;
use engine_core::ecs::item::ItemRegistry;
use engine_core::ecs::template::UnitTemplate;
use engine_core::ecs::world::World;
use engine_core::ecs::world::loadout::validate_template_equipment;
use serde_json::json;
use std::collections::HashMap;

fn setup_world() -> World {
    world_helper::make_test_world()
}

fn register_test_items(world: &mut World) {
    let sword = json!({
        "id": "iron_sword",
        "name": "Iron Sword",
        "slot": "right_hand"
    });
    let shield = json!({
        "id": "iron_shield",
        "name": "Iron Shield",
        "slot": "shield"
    });
    let chainmail = json!({
        "id": "chainmail",
        "name": "Chainmail",
        "slot": "chest"
    });
    world.item_registry.register_item(sword).unwrap();
    world.item_registry.register_item(shield).unwrap();
    world.item_registry.register_item(chainmail).unwrap();
}

fn register_test_set(world: &mut World) {
    let mut items = HashMap::new();
    items.insert("right_hand".to_string(), "iron_sword".to_string());
    items.insert("shield".to_string(), "iron_shield".to_string());
    items.insert("chest".to_string(), "chainmail".to_string());
    world.equipment_set_registry.register_set(EquipmentSet {
        name: "warrior_heavy".to_string(),
        version: "1.0.0".to_string(),
        description: "Heavy warrior loadout".to_string(),
        items,
    });
}

fn create_entity_with_inventory(world: &mut World) -> u32 {
    let eid = world.spawn_entity();
    let inv = json!({
        "slots": ["iron_sword", "iron_shield", "chainmail"],
        "weight": 0.0,
        "volume": 0.0
    });
    world.set_component(eid, "Inventory", inv).unwrap();
    eid
}

#[test]
fn test_apply_loadout_success() {
    let mut world = setup_world();
    register_test_items(&mut world);
    register_test_set(&mut world);
    let eid = create_entity_with_inventory(&mut world);

    let result = world.apply_loadout(eid, "warrior_heavy");
    assert!(result.is_ok());

    let equipment = world.get_component(eid, "Equipment").unwrap();
    let slots = equipment.get("slots").unwrap().as_object().unwrap();
    assert_eq!(slots.get("right_hand").unwrap(), "iron_sword");
    assert_eq!(slots.get("shield").unwrap(), "iron_shield");
    assert_eq!(slots.get("chest").unwrap(), "chainmail");
}

#[test]
fn test_apply_loadout_set_not_found() {
    let mut world = setup_world();
    let eid = create_entity_with_inventory(&mut world);

    let result = world.apply_loadout(eid, "nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_apply_loadout_no_inventory() {
    let mut world = setup_world();
    register_test_items(&mut world);
    register_test_set(&mut world);
    let eid = world.spawn_entity();

    let result = world.apply_loadout(eid, "warrior_heavy");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Inventory"));
}

#[test]
fn test_apply_loadout_unregistered_item() {
    let mut world = setup_world();
    register_test_set(&mut world); // set references items, but items not registered
    let eid = create_entity_with_inventory(&mut world);

    let result = world.apply_loadout(eid, "warrior_heavy");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found in registry"));
}

#[test]
fn test_apply_loadout_creates_item_entities() {
    let mut world = setup_world();
    register_test_items(&mut world);
    register_test_set(&mut world);
    let eid = create_entity_with_inventory(&mut world);

    let initial_entity_count = world.entities.len();
    world.apply_loadout(eid, "warrior_heavy").unwrap();

    // 3 items should have been spawned (iron_sword, iron_shield, chainmail)
    assert_eq!(world.entities.len(), initial_entity_count + 3);

    // Each spawned entity should have an Item component
    let item_entities: Vec<u32> = world
        .entities
        .iter()
        .filter(|&&e| e != eid && world.get_component(e, "Item").is_some())
        .copied()
        .collect();
    assert_eq!(item_entities.len(), 3);
}

#[test]
fn test_validate_equipment_valid() {
    let mut world = setup_world();
    register_test_items(&mut world);
    let eid = create_entity_with_inventory(&mut world);

    let equipment = json!({
        "slots": {
            "right_hand": "iron_sword",
            "chest": "chainmail"
        }
    });
    world.set_component(eid, "Equipment", equipment).unwrap();

    let issues = world.validate_equipment(eid);
    assert!(issues.is_empty());
}

#[test]
fn test_validate_equipment_unknown_slot() {
    let mut world = setup_world();
    register_test_items(&mut world);
    let eid = world.spawn_entity();

    let equipment = json!({
        "slots": {
            "bogus_slot": "iron_sword"
        }
    });
    world.set_component(eid, "Equipment", equipment).unwrap();

    let issues = world.validate_equipment(eid);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].reason, "unknown_slot");
    assert_eq!(issues[0].slot, "bogus_slot");
}

#[test]
fn test_validate_equipment_unregistered_item() {
    let mut world = setup_world();
    let eid = world.spawn_entity();

    let equipment = json!({
        "slots": {
            "right_hand": "nonexistent_sword"
        }
    });
    world.set_component(eid, "Equipment", equipment).unwrap();

    let issues = world.validate_equipment(eid);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].reason, "item_not_registered");
    assert_eq!(issues[0].item_id, "nonexistent_sword");
}

#[test]
fn test_validate_equipment_slot_mismatch() {
    let mut world = setup_world();
    world
        .item_registry
        .register_item(json!({
            "id": "iron_sword",
            "name": "Iron Sword",
            "slot": "right_hand"
        }))
        .unwrap();

    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "chest": "iron_sword"
        }
    });
    world.set_component(eid, "Equipment", equipment).unwrap();

    let issues = world.validate_equipment(eid);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].reason, "slot_mismatch");
}

#[test]
fn test_validate_template_equipment_missing_items() {
    let mut reg = ItemRegistry::new();
    reg.register_item(json!({
        "id": "iron_sword",
        "name": "Iron Sword",
        "slot": "right_hand"
    }))
    .unwrap();

    let template = UnitTemplate {
        name: "warrior".to_string(),
        version: "1.0.0".to_string(),
        description: "Test warrior".to_string(),
        components: {
            let mut m = HashMap::new();
            m.insert(
                "Equipment".to_string(),
                json!({
                    "slots": {
                        "right_hand": "iron_sword",
                        "shield": "iron_shield"
                    }
                }),
            );
            m
        },
    };

    let warnings = validate_template_equipment(&template, &reg);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].item_id, "iron_shield");
    assert_eq!(warnings[0].slot, "shield");
    assert!(warnings[0].message.contains("not found in registry"));
}

#[test]
fn test_validate_template_equipment_all_valid() {
    let mut reg = ItemRegistry::new();
    reg.register_item(json!({
        "id": "iron_sword",
        "name": "Iron Sword",
        "slot": "right_hand"
    }))
    .unwrap();

    let template = UnitTemplate {
        name: "warrior".to_string(),
        version: "1.0.0".to_string(),
        description: "Test warrior".to_string(),
        components: {
            let mut m = HashMap::new();
            m.insert(
                "Equipment".to_string(),
                json!({
                    "slots": {
                        "right_hand": "iron_sword"
                    }
                }),
            );
            m
        },
    };

    let warnings = validate_template_equipment(&template, &reg);
    assert!(warnings.is_empty());
}

#[test]
fn test_validate_template_equipment_no_equipment_component() {
    let reg = ItemRegistry::new();
    let template = UnitTemplate {
        name: "basic".to_string(),
        version: "1.0.0".to_string(),
        description: "No equipment".to_string(),
        components: HashMap::new(),
    };

    let warnings = validate_template_equipment(&template, &reg);
    assert!(warnings.is_empty());
}

#[test]
fn test_spawn_from_template_with_invalid_equipment_succeeds() {
    let mut world = setup_world();

    let mut components = HashMap::new();
    components.insert(
        "Equipment".to_string(),
        json!({
            "slots": {
                "right_hand": "nonexistent_sword"
            }
        }),
    );
    world.template_registry.register_template(UnitTemplate {
        name: "warrior_bad_equip".to_string(),
        version: "1.0.0".to_string(),
        description: "Warrior with bad equipment refs".to_string(),
        components,
    });

    let eid = world
        .spawn_from_template("warrior_bad_equip", None)
        .unwrap();
    let equipment = world.get_component(eid, "Equipment").unwrap();
    assert_eq!(equipment["slots"]["right_hand"], "nonexistent_sword");
}
