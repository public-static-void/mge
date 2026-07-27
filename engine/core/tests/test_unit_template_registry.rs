use engine_core::ecs::template::{UnitTemplate, UnitTemplateRegistry};
use std::collections::HashMap;
use tempfile::tempdir;

fn make_template(name: &str) -> UnitTemplate {
    let mut components = HashMap::new();
    components.insert(
        "Health".to_string(),
        serde_json::json!({"current": 50, "max": 50}),
    );
    UnitTemplate {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: format!("Test {name}"),
        components,
    }
}

#[test]
fn test_register_and_get_template() {
    let mut reg = UnitTemplateRegistry::new();
    let tmpl = make_template("warrior");
    reg.register_template(tmpl);
    assert!(reg.get_template("warrior").is_some());
    assert_eq!(reg.get_template("warrior").unwrap().version, "1.0.0");
}

#[test]
fn test_list_templates() {
    let mut reg = UnitTemplateRegistry::new();
    reg.register_template(make_template("warrior"));
    reg.register_template(make_template("archer"));
    reg.register_template(make_template("worker"));
    let mut names = reg.list_templates();
    names.sort();
    assert_eq!(names, vec!["archer", "warrior", "worker"]);
}

#[test]
fn test_duplicate_name_overrides() {
    let mut reg = UnitTemplateRegistry::new();
    reg.register_template(make_template("warrior"));
    let mut tmpl2 = make_template("warrior");
    tmpl2.version = "2.0.0".to_string();
    reg.register_template(tmpl2);
    assert_eq!(reg.get_template("warrior").unwrap().version, "2.0.0");
}

#[test]
fn test_load_from_dir() {
    let dir = tempdir().unwrap();
    let tmpl = make_template("warrior");
    let json_str = serde_json::to_string_pretty(&tmpl).unwrap();
    std::fs::write(dir.path().join("warrior.json"), json_str).unwrap();

    let mut reg = UnitTemplateRegistry::new();
    reg.load_templates_from_dir(dir.path()).unwrap();
    assert!(reg.get_template("warrior").is_some());
}
