use engine_core::ecs::equipment_set::{EquipmentSet, EquipmentSetRegistry};
use std::collections::HashMap;
use tempfile::tempdir;

fn make_set(name: &str) -> EquipmentSet {
    let mut items = HashMap::new();
    items.insert("right_hand".to_string(), "iron_sword".to_string());
    items.insert("chest".to_string(), "chainmail".to_string());
    EquipmentSet {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: format!("Test {name}"),
        items,
    }
}

#[test]
fn test_register_and_get_set() {
    let mut reg = EquipmentSetRegistry::new();
    let set = make_set("warrior_heavy");
    reg.register_set(set);
    assert!(reg.get_set("warrior_heavy").is_some());
    assert_eq!(reg.get_set("warrior_heavy").unwrap().version, "1.0.0");
}

#[test]
fn test_list_sets() {
    let mut reg = EquipmentSetRegistry::new();
    reg.register_set(make_set("warrior_heavy"));
    reg.register_set(make_set("archer_light"));
    reg.register_set(make_set("mage_robes"));
    let mut names = reg.list_sets();
    names.sort();
    assert_eq!(names, vec!["archer_light", "mage_robes", "warrior_heavy"]);
}

#[test]
fn test_duplicate_name_overrides() {
    let mut reg = EquipmentSetRegistry::new();
    reg.register_set(make_set("warrior_heavy"));
    let mut set2 = make_set("warrior_heavy");
    set2.version = "2.0.0".to_string();
    reg.register_set(set2);
    assert_eq!(reg.get_set("warrior_heavy").unwrap().version, "2.0.0");
}

#[test]
fn test_load_from_dir() {
    let dir = tempdir().unwrap();
    let set = make_set("warrior_heavy");
    let json_str = serde_json::to_string_pretty(&set).unwrap();
    std::fs::write(dir.path().join("warrior_heavy.json"), json_str).unwrap();

    let mut reg = EquipmentSetRegistry::new();
    reg.load_sets_from_dir(dir.path()).unwrap();
    assert!(reg.get_set("warrior_heavy").is_some());
}

#[test]
fn test_default_version() {
    let set: EquipmentSet =
        serde_json::from_str(r#"{"name": "test", "items": {"right_hand": "sword"}}"#).unwrap();
    assert_eq!(set.version, "1.0.0");
    assert_eq!(set.description, "");
}
