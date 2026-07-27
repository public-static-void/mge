use engine_core::ecs::world::wasm::WasmWorld;
use tempfile::tempdir;

fn make_world() -> WasmWorld {
    let mut world = WasmWorld::new();
    let schema_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("engine")
        .join("assets")
        .join("schemas");
    if schema_dir.exists() {
        let schemas = engine_core::ecs::world::wasm::load_schemas_from_dir(&schema_dir);
        world.component_schemas = schemas;
    }
    world
}

fn make_entity_with_inventory(world: &mut WasmWorld) -> u32 {
    let e = world.spawn_entity();
    let inv = serde_json::json!({"slots": [], "weight": 0.0, "volume": 0.0});
    let inv_json = serde_json::to_string(&inv).unwrap();
    world.set_component(e, "Inventory", &inv_json).unwrap();
    e
}

fn register_test_items(world: &mut WasmWorld) {
    let sword = serde_json::json!({"id": "iron_sword", "name": "Iron Sword", "slot": "right_hand"});
    let shield = serde_json::json!({"id": "iron_shield", "name": "Iron Shield", "slot": "shield"});
    world
        .register_item_from_json(&serde_json::to_string(&sword).unwrap())
        .unwrap();
    world
        .register_item_from_json(&serde_json::to_string(&shield).unwrap())
        .unwrap();
}

// --- Item Definition Tests ---

#[test]
fn test_register_and_get_item() {
    let mut world = make_world();
    let item_json = serde_json::json!({"id": "sword", "name": "Sword", "slot": "right_hand"});
    world
        .register_item_from_json(&serde_json::to_string(&item_json).unwrap())
        .unwrap();

    let result = world.get_item_definition_json("sword");
    assert!(result.is_some());
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(parsed["id"], "sword");
    assert_eq!(parsed["name"], "Sword");
}

#[test]
fn test_get_item_nonexistent() {
    let world = make_world();
    assert!(world.get_item_definition_json("nonexistent").is_none());
}

#[test]
fn test_list_item_definitions() {
    let mut world = make_world();
    register_test_items(&mut world);
    let ids = world.list_item_names();
    assert!(ids.contains(&"iron_sword".to_string()));
    assert!(ids.contains(&"iron_shield".to_string()));
}

#[test]
fn test_load_item_definitions_from_dir() {
    let dir = tempdir().unwrap();
    let items = vec![
        (
            "sword.json",
            serde_json::json!({"id": "sword", "name": "Sword", "slot": "right_hand"}),
        ),
        (
            "shield.json",
            serde_json::json!({"id": "shield", "name": "Shield", "slot": "shield"}),
        ),
    ];
    for (name, data) in &items {
        let path = dir.path().join(name);
        std::fs::write(&path, serde_json::to_string(data).unwrap()).unwrap();
    }

    let mut world = make_world();
    world
        .load_item_definitions_from_dir(dir.path().to_str().unwrap())
        .unwrap();
    assert!(world.get_item_definition_json("sword").is_some());
    assert!(world.get_item_definition_json("shield").is_some());
}

// --- Equipment Set Tests ---

#[test]
fn test_register_and_get_equipment_set() {
    let mut world = make_world();
    let set = serde_json::json!({
        "name": "warrior",
        "version": "1.0.0",
        "description": "",
        "items": {"right_hand": "sword"}
    });
    world
        .register_equipment_set_from_json(&serde_json::to_string(&set).unwrap())
        .unwrap();

    // Verify it's registered by listing
    let names = world.equipment_set_registry.list_sets();
    assert!(names.contains(&"warrior".to_string()));
}

#[test]
fn test_load_equipment_sets_from_dir() {
    let dir = tempdir().unwrap();
    let set = serde_json::json!({
        "name": "warrior",
        "version": "1.0.0",
        "description": "",
        "items": {"right_hand": "sword"}
    });
    let path = dir.path().join("warrior.json");
    std::fs::write(&path, serde_json::to_string(&set).unwrap()).unwrap();

    let mut world = make_world();
    world
        .load_equipment_sets_from_dir(dir.path().to_str().unwrap())
        .unwrap();
    let names = world.equipment_set_registry.list_sets();
    assert!(names.contains(&"warrior".to_string()));
}

// --- Apply Loadout Tests ---

#[test]
fn test_apply_loadout_success() {
    let mut world = make_world();
    register_test_items(&mut world);

    let set = serde_json::json!({
        "name": "warrior",
        "version": "1.0.0",
        "description": "",
        "items": {"right_hand": "iron_sword"}
    });
    world
        .register_equipment_set_from_json(&serde_json::to_string(&set).unwrap())
        .unwrap();

    let e = make_entity_with_inventory(&mut world);
    let result = world.apply_loadout(e, "warrior");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), e);

    // Verify equipment slot is set
    let eq = world.get_component(e, "Equipment").unwrap();
    let eq_val: serde_json::Value = serde_json::from_str(&eq).unwrap();
    assert_eq!(eq_val["slots"]["right_hand"], "iron_sword");
}

#[test]
fn test_apply_loadout_unregistered_item() {
    let mut world = make_world();

    let set = serde_json::json!({
        "name": "bad_set",
        "version": "1.0.0",
        "description": "",
        "items": {"right_hand": "nonexistent"}
    });
    world
        .register_equipment_set_from_json(&serde_json::to_string(&set).unwrap())
        .unwrap();

    let e = make_entity_with_inventory(&mut world);
    let result = world.apply_loadout(e, "bad_set");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found in registry"));
}

#[test]
fn test_apply_loadout_no_inventory() {
    let mut world = make_world();
    register_test_items(&mut world);

    let set = serde_json::json!({
        "name": "warrior",
        "version": "1.0.0",
        "description": "",
        "items": {"right_hand": "iron_sword"}
    });
    world
        .register_equipment_set_from_json(&serde_json::to_string(&set).unwrap())
        .unwrap();

    let e = world.spawn_entity(); // no inventory
    let result = world.apply_loadout(e, "warrior");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no Inventory"));
}

#[test]
fn test_apply_loadout_creates_item_entities() {
    let mut world = make_world();
    register_test_items(&mut world);

    let set = serde_json::json!({
        "name": "warrior",
        "version": "1.0.0",
        "description": "",
        "items": {"right_hand": "iron_sword", "shield": "iron_shield"}
    });
    world
        .register_equipment_set_from_json(&serde_json::to_string(&set).unwrap())
        .unwrap();

    let e = make_entity_with_inventory(&mut world);
    world.apply_loadout(e, "warrior").unwrap();

    // Item entities should have Item component
    let items = world.get_entities_with_component("Item");
    assert_eq!(items.len(), 2);
}

// --- Get Loadout Tests ---

#[test]
fn test_get_loadout_match() {
    let mut world = make_world();
    register_test_items(&mut world);

    let set = serde_json::json!({
        "name": "warrior",
        "version": "1.0.0",
        "description": "",
        "items": {"right_hand": "iron_sword"}
    });
    world
        .register_equipment_set_from_json(&serde_json::to_string(&set).unwrap())
        .unwrap();

    let e = make_entity_with_inventory(&mut world);
    world.apply_loadout(e, "warrior").unwrap();

    let result = world.get_loadout_json(e);
    assert!(result.is_some());
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(parsed["name"], "warrior");
}

#[test]
fn test_get_loadout_no_match() {
    let mut world = make_world();
    register_test_items(&mut world);

    let set = serde_json::json!({
        "name": "warrior",
        "version": "1.0.0",
        "description": "",
        "items": {"right_hand": "iron_sword"}
    });
    world
        .register_equipment_set_from_json(&serde_json::to_string(&set).unwrap())
        .unwrap();

    let e = make_entity_with_inventory(&mut world);
    // Don't apply loadout — no equipment
    let result = world.get_loadout_json(e);
    assert!(result.is_none());
}

// --- Validate Equipment Tests ---

#[test]
fn test_validate_equipment_valid() {
    let mut world = make_world();
    register_test_items(&mut world);

    let e = world.spawn_entity();
    let eq = serde_json::json!({"slots": {"right_hand": "iron_sword"}});
    world
        .set_component(e, "Equipment", &serde_json::to_string(&eq).unwrap())
        .unwrap();

    let issues = world.validate_equipment(e);
    assert!(issues.is_empty());
}

#[test]
fn test_validate_equipment_unknown_item() {
    let mut world = make_world();

    let e = world.spawn_entity();
    let eq = serde_json::json!({"slots": {"right_hand": "unknown_item"}});
    world
        .set_component(e, "Equipment", &serde_json::to_string(&eq).unwrap())
        .unwrap();

    let issues = world.validate_equipment(e);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].reason, "item_not_registered");
    assert_eq!(issues[0].item_id, "unknown_item");
}

#[test]
fn test_validate_equipment_empty() {
    let mut world = make_world();

    let e = world.spawn_entity();
    let eq = serde_json::json!({"slots": {}});
    world
        .set_component(e, "Equipment", &serde_json::to_string(&eq).unwrap())
        .unwrap();

    let issues = world.validate_equipment(e);
    assert!(issues.is_empty());
}
