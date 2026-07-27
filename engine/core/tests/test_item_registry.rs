use engine_core::ecs::item::ItemRegistry;
use serde_json::json;
use tempfile::tempdir;

fn make_item(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": format!("Test {id}"),
        "slot": "right_hand"
    })
}

#[test]
fn test_register_and_get_item() {
    let mut reg = ItemRegistry::new();
    let item = make_item("iron_sword");
    reg.register_item(item).unwrap();
    assert!(reg.get_item("iron_sword").is_some());
    assert_eq!(
        reg.get_item("iron_sword")
            .unwrap()
            .get("name")
            .unwrap()
            .as_str()
            .unwrap(),
        "Test iron_sword"
    );
}

#[test]
fn test_list_items() {
    let mut reg = ItemRegistry::new();
    reg.register_item(make_item("iron_sword")).unwrap();
    reg.register_item(make_item("iron_shield")).unwrap();
    reg.register_item(make_item("chainmail")).unwrap();
    let mut names = reg.list_items();
    names.sort();
    assert_eq!(names, vec!["chainmail", "iron_shield", "iron_sword"]);
}

#[test]
fn test_duplicate_id_overrides() {
    let mut reg = ItemRegistry::new();
    reg.register_item(make_item("iron_sword")).unwrap();
    let mut item2 = make_item("iron_sword");
    item2["name"] = json!("Updated Sword");
    reg.register_item(item2).unwrap();
    assert_eq!(
        reg.get_item("iron_sword")
            .unwrap()
            .get("name")
            .unwrap()
            .as_str()
            .unwrap(),
        "Updated Sword"
    );
}

#[test]
fn test_register_item_missing_id_errors() {
    let mut reg = ItemRegistry::new();
    let item = json!({"name": "Sword", "slot": "right_hand"});
    assert!(reg.register_item(item).is_err());
}

#[test]
fn test_register_item_missing_name_errors() {
    let mut reg = ItemRegistry::new();
    let item = json!({"id": "sword", "slot": "right_hand"});
    assert!(reg.register_item(item).is_err());
}

#[test]
fn test_register_item_missing_slot_errors() {
    let mut reg = ItemRegistry::new();
    let item = json!({"id": "sword", "name": "Sword"});
    assert!(reg.register_item(item).is_err());
}

#[test]
fn test_load_from_dir() {
    let dir = tempdir().unwrap();
    let item = make_item("iron_sword");
    let json_str = serde_json::to_string_pretty(&item).unwrap();
    std::fs::write(dir.path().join("iron_sword.json"), json_str).unwrap();

    let mut reg = ItemRegistry::new();
    reg.load_items_from_dir(dir.path()).unwrap();
    assert!(reg.get_item("iron_sword").is_some());
}

#[test]
fn test_load_from_dir_missing_id_errors() {
    let dir = tempdir().unwrap();
    let item = json!({"name": "Sword", "slot": "right_hand"});
    let json_str = serde_json::to_string_pretty(&item).unwrap();
    std::fs::write(dir.path().join("bad_item.json"), json_str).unwrap();

    let mut reg = ItemRegistry::new();
    assert!(reg.load_items_from_dir(dir.path()).is_err());
}
