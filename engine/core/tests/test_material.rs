use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::world::World;
use engine_core::material::{
    default_material, get_entity_material, get_material_names, get_material_properties,
    set_entity_material,
};
use std::sync::{Arc, Mutex};

fn setup_world_with_materials() -> World {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        // Register the Material schema
        let material_schema: serde_json::Value =
            serde_json::from_str(include_str!("../../assets/schemas/material.json")).unwrap();
        reg.register_external_schema(ComponentSchema {
            name: "Material".to_string(),
            schema: material_schema,
            modes: vec!["colony".to_string(), "roguelike".to_string()],
        });
        // Register the Item schema (with the new optional material field)
        let item_schema: serde_json::Value =
            serde_json::from_str(include_str!("../../assets/schemas/item.json")).unwrap();
        reg.register_external_schema(ComponentSchema {
            name: "Item".to_string(),
            schema: item_schema,
            modes: vec!["colony".to_string(), "roguelike".to_string()],
        });
    }
    let mut world = World::new(registry);
    // Resolve materials dir relative to crate root (engine/core/)
    let materials_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine/assets/materials");
    let mats = engine_core::ecs::assets::load_material_definitions(&materials_dir)
        .expect("Failed to load material definitions");
    world.material_definitions = mats;
    world
}

#[test]
fn test_material_definitions_load() {
    let world = setup_world_with_materials();
    assert_eq!(world.material_definitions.len(), 8);
    assert!(world.material_definitions.contains_key("wood"));
    assert!(world.material_definitions.contains_key("iron"));
    assert!(world.material_definitions.contains_key("steel"));
    assert!(world.material_definitions.contains_key("obsidian"));
}

#[test]
fn test_get_material_properties_wood() {
    let world = setup_world_with_materials();
    let props = get_material_properties(&world, "wood");
    assert_eq!(props["density"], 0.6);
    assert_eq!(props["hardness"], 2.0);
    assert_eq!(props["flammability"], 0.9);
}

#[test]
fn test_get_material_properties_unknown_returns_default() {
    let world = setup_world_with_materials();
    let props = get_material_properties(&world, "nonexistent_material");
    let def = default_material();
    assert_eq!(props["density"], def["density"]);
    assert_eq!(props["hardness"], def["hardness"]);
    assert_eq!(props["flammability"], def["flammability"]);
}

#[test]
fn test_set_entity_material_success() {
    let mut world = setup_world_with_materials();
    let eid = world.spawn_entity();
    set_entity_material(&mut world, eid, "iron").unwrap();
    let comp = get_entity_material(&world, eid).expect("Material component should exist");
    assert_eq!(comp["material"], "iron");
}

#[test]
fn test_set_entity_material_unknown_rejects() {
    let mut world = setup_world_with_materials();
    let eid = world.spawn_entity();
    let result = set_entity_material(&mut world, eid, "nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("nonexistent"));
    assert!(get_entity_material(&world, eid).is_none());
}

#[test]
fn test_get_entity_material_present() {
    let mut world = setup_world_with_materials();
    let eid = world.spawn_entity();
    set_entity_material(&mut world, eid, "steel").unwrap();
    let mat = get_entity_material(&world, eid).unwrap();
    assert_eq!(mat["material"], "steel");
}

#[test]
fn test_get_entity_material_absent() {
    let mut world = setup_world_with_materials();
    let eid = world.spawn_entity();
    assert!(get_entity_material(&world, eid).is_none());
}

#[test]
fn test_get_material_names() {
    let world = setup_world_with_materials();
    let mut names = get_material_names(&world);
    names.sort();
    assert_eq!(
        names,
        vec![
            "bone", "cloth", "iron", "leather", "obsidian", "steel", "stone", "wood"
        ]
    );
}

#[test]
fn test_item_schema_accepts_material() {
    let world = setup_world_with_materials();
    let item = serde_json::json!({
        "id": "sword",
        "name": "Iron Sword",
        "slot": "main_hand",
        "material": "iron"
    });
    let guard = world.registry.lock().unwrap();
    let schema = guard
        .get_schema_by_name("Item")
        .expect("Item schema should be registered");
    let validator = jsonschema::Validator::new(&schema.schema).unwrap();
    assert!(validator.validate(&item).is_ok());
}

#[test]
fn test_item_schema_without_material() {
    let world = setup_world_with_materials();
    let item = serde_json::json!({
        "id": "sword",
        "name": "Iron Sword",
        "slot": "main_hand"
    });
    let guard = world.registry.lock().unwrap();
    let schema = guard
        .get_schema_by_name("Item")
        .expect("Item schema should be registered");
    let validator = jsonschema::Validator::new(&schema.schema).unwrap();
    assert!(validator.validate(&item).is_ok());
}
