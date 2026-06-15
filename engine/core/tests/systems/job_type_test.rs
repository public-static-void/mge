use engine_core::systems::job::types::{JobEffect, JobLogicKind, JobTypeData, JobTypeRegistry};
use serde_json::json;

fn dummy_handler(
    _world: &mut engine_core::ecs::world::World,
    _entity: u32,
    _job_id: u32,
    job_data: &serde_json::Value,
) -> serde_json::Value {
    job_data.clone()
}

fn make_data(name: &str) -> JobTypeData {
    JobTypeData {
        name: name.to_string(),
        requirements: vec!["skill_a".into()],
        duration: Some(5.0),
        effects: vec![json!({"action": "test"})],
    }
}

// --- Registration round-trips ---

#[test]
fn test_register_get_data_native() {
    let mut reg = JobTypeRegistry::new();
    let data = make_data("test_job");
    reg.register(data.clone(), JobLogicKind::Native(dummy_handler));

    let retrieved = reg.get_data("test_job");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "test_job");

    let logic = reg.get_logic("test_job");
    assert!(logic.is_some());
    assert!(matches!(logic.unwrap(), JobLogicKind::Native(_)));
}

#[test]
fn test_register_get_logic_scripted() {
    let mut reg = JobTypeRegistry::new();
    let data = make_data("scripted_job");
    reg.register(data, JobLogicKind::Scripted("my_key".into()));

    let retrieved = reg.get_data("scripted_job");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "scripted_job");

    let logic = reg.get_logic("scripted_job");
    assert!(logic.is_some());
    match logic.unwrap() {
        JobLogicKind::Scripted(key) => assert_eq!(key, "my_key"),
        _ => panic!("Expected Scripted"),
    }
}

// --- Convenience registration ---

#[test]
fn test_register_native_creates_defaults() {
    let mut reg = JobTypeRegistry::new();
    reg.register_native("default_job", dummy_handler);

    let data = reg.get_data("default_job");
    assert!(data.is_some());
    let d = data.unwrap();
    assert_eq!(d.name, "default_job");
    assert!(d.requirements.is_empty());
    assert!(d.duration.is_none());
    assert!(d.effects.is_empty());

    let logic = reg.get_logic("default_job");
    assert!(matches!(logic.unwrap(), JobLogicKind::Native(_)));
}

#[test]
fn test_register_lua() {
    let mut reg = JobTypeRegistry::new();
    reg.register_lua("lua_job", "lua_key".into());

    // register_lua only inserts into the logic map
    assert!(reg.get_data("lua_job").is_none());

    let logic = reg.get_logic("lua_job");
    assert!(logic.is_some());
    match logic.unwrap() {
        JobLogicKind::Scripted(key) => assert_eq!(key, "lua_key"),
        _ => panic!("Expected Scripted"),
    }
}

#[test]
fn test_register_job_type() {
    let mut reg = JobTypeRegistry::new();
    let data = make_data("auto_job");
    reg.register_job_type(data);

    let retrieved = reg.get_data("auto_job");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "auto_job");

    let logic = reg.get_logic("auto_job");
    assert!(matches!(logic.unwrap(), JobLogicKind::Native(_)));
}

// --- Name enumeration ---

#[test]
fn test_job_type_names() {
    let mut reg = JobTypeRegistry::new();
    reg.register_native("alpha", dummy_handler);
    reg.register_native("beta", dummy_handler);

    let mut names = reg.job_type_names();
    names.sort();
    assert_eq!(names, vec!["alpha", "beta"]);
}

#[test]
fn test_job_type_names_uses_data_map() {
    let mut reg = JobTypeRegistry::new();
    // register_lua only adds to logic, not data
    reg.register_lua("only_logic", "key".into());
    assert!(reg.job_type_names().is_empty());

    // registering via register adds to both
    reg.register_native("both", dummy_handler);
    assert_eq!(reg.job_type_names(), vec!["both"]);
}

#[test]
fn test_empty_registry_no_names() {
    let reg = JobTypeRegistry::new();
    assert!(reg.job_type_names().is_empty());
}

// --- Error / edge paths ---

#[test]
fn test_get_data_unregistered() {
    let reg = JobTypeRegistry::new();
    assert!(reg.get_data("nonexistent").is_none());
}

#[test]
fn test_get_logic_unregistered() {
    let reg = JobTypeRegistry::new();
    assert!(reg.get_logic("nonexistent").is_none());
}

#[test]
fn test_name_in_logic_only() {
    let mut reg = JobTypeRegistry::new();
    reg.register_lua("logic_only", "key".into());
    assert!(reg.get_data("logic_only").is_none());
    assert!(reg.get_logic("logic_only").is_some());
}

#[test]
fn test_register_overwrite() {
    let mut reg = JobTypeRegistry::new();
    let first = make_data("overwrite");
    let mut second = make_data("overwrite");
    second.duration = Some(99.0);

    reg.register(first, JobLogicKind::Native(dummy_handler));
    reg.register(second, JobLogicKind::Scripted("new".into()));

    let data = reg.get_data("overwrite").unwrap();
    assert_eq!(data.duration, Some(99.0));

    let logic = reg.get_logic("overwrite").unwrap();
    assert!(matches!(logic, JobLogicKind::Scripted(_)));
}

// --- Deserialization round-trips ---

#[test]
fn test_job_type_data_roundtrip_json() {
    let data = make_data("roundtrip_json");
    let json = serde_json::to_string(&data).unwrap();
    let deserialized: JobTypeData = serde_json::from_str(&json).unwrap();
    assert_eq!(data, deserialized);
}

#[test]
fn test_job_type_data_roundtrip_toml() {
    let data = make_data("roundtrip_toml");
    let toml_str = toml::to_string(&data).unwrap();
    let deserialized: JobTypeData = toml::from_str(&toml_str).unwrap();
    assert_eq!(data, deserialized);
}

#[test]
fn test_job_effect_deserialize() {
    let json = r#"{"action": "move", "from": "A", "to": "B"}"#;
    let effect: JobEffect = serde_json::from_str(json).unwrap();
    assert_eq!(effect.action, "move");
    assert_eq!(effect.from, Some("A".to_string()));
    assert_eq!(effect.to, Some("B".to_string()));
}

// --- Default-value edge cases ---

#[test]
fn test_job_type_data_defaults() {
    // minimal JSON — missing optional fields
    let json = r#"{"name": "minimal"}"#;
    let data: JobTypeData = serde_json::from_str(json).unwrap();
    assert_eq!(data.name, "minimal");
    assert!(data.requirements.is_empty());
    assert!(data.duration.is_none());
    assert!(data.effects.is_empty());
}
