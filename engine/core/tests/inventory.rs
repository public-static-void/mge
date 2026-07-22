#[path = "helpers/world.rs"]
mod world_helper;

#[path = "helpers/inventory.rs"]
mod inventory_helper;

use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::loot::{LootEntry, LootTableRegistry};
use engine_core::systems::inventory::InventoryConstraintSystem;
use inventory_helper::create_inventory;
use serde_json::json;
use std::sync::{Arc, Mutex};
use world_helper::make_test_world;

// === inventory_system tests ===

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

    let mut updated = world.get_component(eid, "Inventory").unwrap().clone();
    let mut slots = updated["slots"].as_array().unwrap().clone();
    slots.push(json!("item_3"));
    updated["slots"] = json!(slots);
    updated["weight"] = json!(4.0);
    updated["volume"] = json!(2.5);
    world.set_component(eid, "Inventory", updated).unwrap();

    let mut overfilled = world.get_component(eid, "Inventory").unwrap().clone();
    let mut slots = overfilled["slots"].as_array().unwrap().clone();
    slots.push(json!("item_4"));
    slots.push(json!("item_5"));
    slots.push(json!("item_6"));
    overfilled["slots"] = json!(slots);
    let _ = world.set_component(eid, "Inventory", overfilled);

    world.run_system("InventoryConstraintSystem").unwrap();

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

    let mut inv = world.get_component(eid, "Inventory").unwrap().clone();
    let mut slots = inv["slots"].as_array().unwrap().clone();
    slots.push(json!("item_3"));
    inv["slots"] = json!(slots);
    world.set_component(eid, "Inventory", inv).unwrap();

    world.run_system("InventoryConstraintSystem").unwrap();

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

    world.run_system("InventoryConstraintSystem").unwrap();

    let inv_after = world.get_component(eid, "Inventory").unwrap();
    assert_eq!(inv_after["encumbered"], true);
}

// === stockpile_management tests ===

#[test]
fn test_add_and_remove_stockpile_resources() {
    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");

    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }

    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    let entity = world.spawn_entity();

    let res = world.set_component(
        entity,
        "Stockpile",
        json!({
            "resources": { "wood": 10, "stone": 5 }
        }),
    );
    assert!(res.is_ok(), "Failed to add stockpile: {res:?}");

    let res = world.modify_stockpile_resource(entity, "food", 3.0);
    assert!(res.is_ok());

    let res = world.modify_stockpile_resource(entity, "wood", -2.0);
    assert!(res.is_ok());

    let stockpile = world.get_component(entity, "Stockpile").unwrap();
    assert_eq!(stockpile["resources"]["wood"], 8.0);
    assert_eq!(stockpile["resources"]["stone"], 5.0);
    assert_eq!(stockpile["resources"]["food"], 3.0);

    let res = world.modify_stockpile_resource(entity, "wood", -20.0);
    assert!(
        res.is_err(),
        "Should not be able to remove more than available"
    );
}

// === loot tests ===

#[test]
fn test_define_and_roll() {
    let mut registry = LootTableRegistry::new();
    registry
        .define_table(
            "test",
            vec![LootEntry {
                item_id: "item1".into(),
                weight: 100,
                min_count: 1,
                max_count: 1,
            }],
        )
        .unwrap();

    let result = registry.roll("test").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "item1");
    assert_eq!(result[0].1, 1);
}

#[test]
fn test_empty_table_returns_error() {
    let mut registry = LootTableRegistry::new();
    registry.define_table("empty", vec![]).unwrap();
    let result = registry.roll("empty");
    assert!(result.is_err());
}

#[test]
fn test_undefined_table_returns_error() {
    let registry = LootTableRegistry::new();
    let result = registry.roll("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_zero_weight_validated() {
    let mut registry = LootTableRegistry::new();
    let result = registry.define_table(
        "bad",
        vec![LootEntry {
            item_id: "item".into(),
            weight: 0,
            min_count: 1,
            max_count: 1,
        }],
    );
    assert!(result.is_err());
}

#[test]
fn test_min_max_count_range() {
    let mut registry = LootTableRegistry::new();
    registry
        .define_table(
            "multi",
            vec![LootEntry {
                item_id: "coins".into(),
                weight: 100,
                min_count: 2,
                max_count: 5,
            }],
        )
        .unwrap();

    for _ in 0..20 {
        let result = registry.roll("multi").unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].1 >= 2 && result[0].1 <= 5);
    }
}

#[test]
fn test_weighted_distribution() {
    let mut registry = LootTableRegistry::new();
    registry
        .define_table(
            "weighted",
            vec![
                LootEntry {
                    item_id: "common".into(),
                    weight: 90,
                    min_count: 1,
                    max_count: 1,
                },
                LootEntry {
                    item_id: "rare".into(),
                    weight: 10,
                    min_count: 1,
                    max_count: 1,
                },
            ],
        )
        .unwrap();

    let mut common_count = 0u32;
    let mut rare_count = 0u32;
    let total_rolls = 100;
    for _ in 0..total_rolls {
        let result = registry.roll("weighted").unwrap();
        assert_eq!(result.len(), 1, "weighted-sum should return exactly 1 item");
        if result[0].0 == "common" {
            common_count += 1;
        } else {
            rare_count += 1;
        }
    }
    assert_eq!(
        common_count + rare_count,
        total_rolls,
        "every roll should produce exactly one item"
    );
    assert!(
        common_count > rare_count,
        "common (weight 90) should be selected more often than rare (weight 10): had {} vs {}",
        common_count,
        rare_count
    );
}

#[test]
fn test_has_table() {
    let mut registry = LootTableRegistry::new();
    assert!(!registry.has_table("foo"));
    registry
        .define_table(
            "foo",
            vec![LootEntry {
                item_id: "bar".into(),
                weight: 100,
                min_count: 1,
                max_count: 1,
            }],
        )
        .unwrap();
    assert!(registry.has_table("foo"));
}

#[test]
fn test_table_names() {
    let mut registry = LootTableRegistry::new();
    registry.define_table("a", vec![]).unwrap();
    registry.define_table("b", vec![]).unwrap();
    let names = registry.table_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"b".to_string()));
}

#[test]
fn test_remove_table() {
    let mut registry = LootTableRegistry::new();
    registry
        .define_table(
            "temp",
            vec![LootEntry {
                item_id: "x".into(),
                weight: 100,
                min_count: 1,
                max_count: 1,
            }],
        )
        .unwrap();
    assert!(registry.has_table("temp"));
    registry.remove_table("temp");
    assert!(!registry.has_table("temp"));
    assert!(registry.roll("temp").is_err());
}

#[test]
fn test_invalid_min_max_count() {
    let mut registry = LootTableRegistry::new();
    let result = registry.define_table(
        "bad",
        vec![LootEntry {
            item_id: "x".into(),
            weight: 100,
            min_count: 5,
            max_count: 1,
        }],
    );
    assert!(result.is_err());
}

#[test]
fn test_define_overwrites() {
    let mut registry = LootTableRegistry::new();
    registry
        .define_table(
            "dupe",
            vec![LootEntry {
                item_id: "old".into(),
                weight: 100,
                min_count: 1,
                max_count: 1,
            }],
        )
        .unwrap();
    registry
        .define_table(
            "dupe",
            vec![LootEntry {
                item_id: "new".into(),
                weight: 100,
                min_count: 1,
                max_count: 1,
            }],
        )
        .unwrap();
    let result = registry.roll("dupe").unwrap();
    assert_eq!(result[0].0, "new");
}
