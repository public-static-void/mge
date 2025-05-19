use engine_core::ecs::Component;
use engine_core::ecs::components::position::{Position, PositionComponent};
use engine_core::ecs::{ComponentRegistry, Health};
use engine_core::systems::inventory::InventoryConstraintSystem;
use semver::Version;

#[test]
fn test_component_registration() {
    let mut registry = ComponentRegistry::new();
    registry.register::<PositionComponent>().unwrap();
    assert!(registry.get_schema::<PositionComponent>().is_some());

    let json = registry.schema_to_json::<PositionComponent>().unwrap();
    println!("Position schema: {}", json);
    assert!(
        json.contains("\"x\""),
        "Schema does not contain field 'x':\n{}",
        json
    );
    assert!(
        json.contains("Position"),
        "Schema does not mention 'Position':\n{}",
        json
    );
}

#[test]
fn test_health_component() {
    let mut registry = ComponentRegistry::new();
    registry.register::<Health>().unwrap();
    assert!(registry.get_schema::<Health>().is_some());

    let json = registry.schema_to_json::<Health>().unwrap();
    println!("Health schema: {}", json);
    assert!(
        json.contains("\"current\""),
        "Schema does not contain field 'current':\n{}",
        json
    );
    assert!(
        json.contains("\"max\""),
        "Schema does not contain field 'max':\n{}",
        json
    );
}

#[test]
fn test_unregistered_component() {
    use engine_core::ecs::RegistryError;

    let registry = ComponentRegistry::new();
    let result = registry.schema_to_json::<Health>();

    match result {
        Ok(_) => panic!("Expected an error, but got Ok"),
        Err(e) => match e {
            RegistryError::UnregisteredComponent => (),
            _ => panic!("Expected UnregisteredComponent error, got {:?}", e),
        },
    }
}

#[test]
fn test_component_migration() {
    use bson::{doc, to_vec};

    // Create test data
    let old_position = doc! { "x": 5.0, "y": 3.0 };
    let data = to_vec(&old_position).unwrap();

    // Perform migration
    let position = PositionComponent::migrate(Version::parse("1.0.0").unwrap(), &data).unwrap();

    if let Position::Square { x, y, .. } = position.pos {
        assert_eq!(x, 5);
        assert_eq!(y, 3);
        // (z as needed)
    } else {
        panic!("Expected Position::Square");
    }
}

#[test]
fn test_version_migration() {
    use engine_core::ecs::MigrationError;
    use semver::Version;

    // Legacy v1 format
    #[derive(serde::Serialize)]
    struct LegacyPosition {
        x: f32,
        y: f32,
    }

    let old_pos = LegacyPosition { x: 5.0, y: 3.0 };
    let data = bson::to_vec(&old_pos).unwrap();

    // Test migration from v1.0.0
    let pos = PositionComponent::migrate(Version::parse("1.0.0").unwrap(), &data).unwrap();
    if let Position::Square { x, y, .. } = pos.pos {
        assert_eq!(x, 5);
        assert_eq!(y, 3);
        // (z as needed)
    } else {
        panic!("Expected Position::Square");
    }

    // Test invalid version
    let result = PositionComponent::migrate(Version::parse("3.0.0").unwrap(), &data);
    assert!(matches!(result, Err(MigrationError::UnsupportedVersion(_))));
}

#[test]
fn test_macro_generated_migration() {
    #[derive(serde::Serialize)]
    struct LegacyPosition {
        x: f32,
        y: f32,
    }

    let data = bson::to_vec(&LegacyPosition { x: 5.0, y: 3.0 }).unwrap();
    let pos = PositionComponent::migrate(Version::parse("1.0.0").unwrap(), &data).unwrap();
    if let Position::Square { x, y, .. } = pos.pos {
        assert_eq!(x, 5);
        assert_eq!(y, 3);
    } else {
        panic!("Expected Position::Square");
    }
}

#[test]
fn test_external_schema_loading() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::schema::load_schemas_from_dir;
    use std::sync::{Arc, Mutex};

    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    assert!(
        schemas.contains_key("Health"),
        "Health schema should be loaded"
    );

    let registry = Arc::new(Mutex::new(ComponentRegistry::default()));

    for (_name, schema) in schemas {
        registry.lock().unwrap().register_external_schema(schema);
    }

    // Now you can check that the registry has the schema
    let guard = registry.lock().unwrap();
    assert!(guard.get_schema_by_name("Health").is_some());
}

#[test]
fn test_schema_driven_mode_enforcement() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    // Fabricate a schema for "Roguelike::Inventory" only allowed in "roguelike" mode
    let roguelike_inventory_schema = r#"
    {
      "title": "Roguelike::Inventory",
      "type": "object",
      "properties": {
        "slots": { "type": "array", "items": { "type": "string" } },
        "weight": { "type": "number" }
      },
      "required": ["slots", "weight"],
      "modes": ["roguelike"]
    }
    "#;

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(roguelike_inventory_schema)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let entity = world.spawn_entity();

    // Not allowed in "colony"
    world.current_mode = "colony".to_string();
    let result = world.set_component(
        entity,
        "Roguelike::Inventory",
        json!({"slots": [], "weight": 0.0}),
    );
    assert!(
        result.is_err(),
        "Roguelike::Inventory should NOT be allowed in colony mode"
    );

    // Allowed in "roguelike"
    world.current_mode = "roguelike".to_string();
    let result = world.set_component(
        entity,
        "Roguelike::Inventory",
        json!({"slots": [], "weight": 0.0}),
    );
    assert!(
        result.is_ok(),
        "Roguelike::Inventory should be allowed in roguelike mode"
    );
}

#[test]
fn test_register_external_schema_from_json() {
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();

    // Example schema JSON string
    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": {
            "mana": { "type": "number", "minimum": 0 }
        },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;

    let result = registry.register_external_schema_from_json(schema_json);
    assert!(
        result.is_ok(),
        "Schema registration failed: {:?}",
        result.err()
    );

    let registry = Arc::new(Mutex::new(registry));

    // FIX: Avoid E0716 by binding the lock guard
    let guard = registry.lock().unwrap();
    let schema = guard.get_schema_by_name("MagicPower");
    assert!(
        schema.is_some(),
        "Schema 'MagicPower' not found in registry"
    );

    // Check modes are correctly set
    let modes = &schema.unwrap().modes;
    assert!(
        modes.contains(&"colony".to_string()),
        "Mode 'colony' not set"
    );
}

#[test]
fn test_mode_enforcement_for_runtime_registered_schema() {
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": { "mana": { "type": "number" } },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let id = world.spawn_entity();

    // Allowed in "colony"
    world.current_mode = "colony".to_string();
    assert!(
        world
            .set_component(id, "MagicPower", json!({ "mana": 42 }))
            .is_ok(),
        "Should be allowed in colony mode"
    );

    // Not allowed in "roguelike"
    world.current_mode = "roguelike".to_string();
    let result = world.set_component(id, "MagicPower", json!({ "mana": 99 }));
    assert!(result.is_err(), "Should not be allowed in roguelike mode");
}

#[test]
fn test_set_component_validation() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let schema_json = r#"
    {
        "title": "TestComponent",
        "type": "object",
        "properties": {
            "value": { "type": "integer", "minimum": 0, "maximum": 10 }
        },
        "required": ["value"],
        "modes": ["colony"]
    }
    "#;
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let entity = world.spawn_entity();

    world.current_mode = "colony".to_string();

    // Valid data
    assert!(
        world
            .set_component(entity, "TestComponent", json!({ "value": 5 }))
            .is_ok()
    );

    // Invalid data (value too high)
    assert!(
        world
            .set_component(entity, "TestComponent", json!({ "value": 20 }))
            .is_err()
    );

    // Missing required field
    assert!(
        world
            .set_component(entity, "TestComponent", json!({}))
            .is_err()
    );
}

#[test]
fn can_register_body_schema_and_assign_body_component() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let body_schema_json = include_str!("../../assets/schemas/body.json");
    registry
        .register_external_schema_from_json(body_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();

    // Create a hierarchical body: torso -> left arm -> left hand
    let eid = world.spawn_entity();
    let body = json!({
        "parts": [
            {
                "name": "torso",
                "kind": "torso",
                "status": "healthy",
                "temperature": 36.5,
                "ideal_temperature": 36.5,
                "insulation": 2.0,
                "heat_loss": 0.1,
                "children": [
                    {
                        "name": "left arm",
                        "kind": "arm",
                        "status": "healthy",
                        "temperature": 35.0,
                        "ideal_temperature": 36.5,
                        "insulation": 1.0,
                        "heat_loss": 0.2,
                        "children": [
                            {
                                "name": "left hand",
                                "kind": "hand",
                                "status": "healthy",
                                "temperature": 34.0,
                                "ideal_temperature": 36.5,
                                "insulation": 0.5,
                                "heat_loss": 0.3,
                                "children": [],
                                "equipped": []
                            }
                        ],
                        "equipped": []
                    }
                ],
                "equipped": []
            }
        ]
    });
    assert!(world.set_component(eid, "Body", body.clone()).is_ok());

    // Query the body component and check the nested structure
    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(stored["parts"][0]["name"], "torso");
    assert_eq!(stored["parts"][0]["children"][0]["name"], "left arm");
    assert_eq!(
        stored["parts"][0]["children"][0]["children"][0]["name"],
        "left hand"
    );
}

#[test]
fn can_update_body_part_status_and_equip_item() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let body_schema_json = include_str!("../../assets/schemas/body.json");
    registry
        .register_external_schema_from_json(body_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();

    // Add a simple body (all required fields present)
    let eid = world.spawn_entity();
    let mut body = json!({
        "parts": [
            {
                "name": "torso",
                "kind": "torso",
                "status": "healthy",
                "temperature": 36.5,
                "ideal_temperature": 36.5,
                "insulation": 2.0,
                "heat_loss": 0.1,
                "children": [
                    {
                        "name": "left arm",
                        "kind": "arm",
                        "status": "healthy",
                        "temperature": 35.0,
                        "ideal_temperature": 36.5,
                        "insulation": 1.0,
                        "heat_loss": 0.2,
                        "children": [],
                        "equipped": []
                    }
                ],
                "equipped": []
            }
        ]
    });
    world.set_component(eid, "Body", body.clone()).unwrap();

    // Wound the left arm
    body["parts"][0]["children"][0]["status"] = json!("wounded");
    world.set_component(eid, "Body", body.clone()).unwrap();
    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(stored["parts"][0]["children"][0]["status"], "wounded");

    // Equip a ring on the left arm
    body["parts"][0]["children"][0]["equipped"] = json!(["gold ring"]);
    world.set_component(eid, "Body", body.clone()).unwrap();
    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(
        stored["parts"][0]["children"][0]["equipped"][0],
        "gold ring"
    );
}

#[test]
fn can_set_and_query_body_part_temperature_and_insulation() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let body_schema_json = include_str!("../../assets/schemas/body.json");
    registry
        .register_external_schema_from_json(body_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();

    let eid = world.spawn_entity();
    let body = json!({
        "parts": [
            {
                "name": "torso",
                "kind": "torso",
                "status": "healthy",
                "temperature": 36.5,
                "ideal_temperature": 36.5,
                "insulation": 2.0,
                "heat_loss": 0.1,
                "children": [
                    {
                        "name": "left arm",
                        "kind": "arm",
                        "status": "healthy",
                        "temperature": 35.0,
                        "ideal_temperature": 36.5,
                        "insulation": 1.0,
                        "heat_loss": 0.2,
                        "children": [
                            {
                                "name": "left hand",
                                "kind": "hand",
                                "status": "healthy",
                                "temperature": 34.0,
                                "ideal_temperature": 36.5,
                                "insulation": 0.5,
                                "heat_loss": 0.3,
                                "children": [],
                                "equipped": []
                            }
                        ],
                        "equipped": ["wool glove"]
                    }
                ],
                "equipped": []
            }
        ]
    });
    assert!(world.set_component(eid, "Body", body.clone()).is_ok());

    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(stored["parts"][0]["temperature"], 36.5);
    assert_eq!(stored["parts"][0]["children"][0]["insulation"], 1.0);
    assert_eq!(
        stored["parts"][0]["children"][0]["equipped"][0],
        "wool glove"
    );
}

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
fn can_register_equipment_schema_and_equip_items() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let equipment_schema_json = include_str!("../../assets/schemas/equipment.json");
    registry
        .register_external_schema_from_json(equipment_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
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
fn cannot_equip_incompatible_item() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::equipment_logic::EquipmentLogicSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let equipment_schema_json = include_str!("../../assets/schemas/equipment.json");
    let item_schema_json = include_str!("../../assets/schemas/item.json");
    registry
        .register_external_schema_from_json(equipment_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(item_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();

    // Register the equipment logic system
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
fn equipping_two_handed_weapon_blocks_both_hands() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::equipment_logic::EquipmentLogicSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let equipment_schema_json = include_str!("../../assets/schemas/equipment.json");
    let item_schema_json = include_str!("../../assets/schemas/item.json");
    registry
        .register_external_schema_from_json(equipment_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(item_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
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
fn cannot_equip_two_handed_weapon_if_other_hand_occupied() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use engine_core::systems::equipment_logic::EquipmentLogicSystem;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let equipment_schema_json = include_str!("../../assets/schemas/equipment.json");
    let item_schema_json = include_str!("../../assets/schemas/item.json");
    registry
        .register_external_schema_from_json(equipment_schema_json)
        .unwrap();
    registry
        .register_external_schema_from_json(item_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
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
fn cannot_equip_item_with_unmet_stat_requirement() {
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
fn can_equip_item_with_met_stat_requirement() {
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
