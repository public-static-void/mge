use engine_core::ecs::schema::{
    ComponentSchema, load_schemas_from_dir, load_schemas_recursively, save_schema_to_file,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_load_schemas_with_validation() {
    let dir = tempdir().unwrap();
    let valid_schema = r#"{
        "title": "ValidComponent",
        "modes": ["colony"],
        "type": "object",
        "properties": { "foo": { "type": "integer" } }
    }"#;
    fs::write(dir.path().join("valid.json"), valid_schema).unwrap();

    // Should succeed
    let schemas = load_schemas_from_dir(dir.path()).unwrap();
    assert!(schemas.contains_key("ValidComponent"));

    // Add an invalid schema
    let invalid_schema = r#"{
        "type": "object",
        "properties": { "foo": { "type": "integer" } }
    }"#; // missing "title" and "modes"
    fs::write(dir.path().join("invalid.json"), invalid_schema).unwrap();

    // Should fail
    let result = load_schemas_from_dir(dir.path());
    assert!(result.is_err());
}

#[test]
fn test_save_and_load_schema_roundtrip() {
    let dir = tempdir().unwrap();
    let schema = ComponentSchema {
        name: "RoundTrip".to_string(),
        schema: serde_json::json!({
            "title": "RoundTrip",
            "modes": ["colony"],
            "type": "object"
        }),
        modes: vec!["colony".to_string()],
    };
    let path = dir.path().join("roundtrip.json");
    save_schema_to_file(&schema, &path).unwrap();

    let loaded = engine_core::ecs::schema::load_schemas_from_dir(dir.path()).unwrap();
    assert!(loaded.contains_key("RoundTrip"));
}

#[test]
fn test_load_schemas_recursively() {
    let dir = tempdir().unwrap();
    let subdir = dir.path().join("nested");
    fs::create_dir(&subdir).unwrap();

    let schema1 = r#"{
        "title": "RootComponent",
        "modes": ["colony"],
        "type": "object"
    }"#;
    let schema2 = r#"{
        "title": "NestedComponent",
        "modes": ["colony"],
        "type": "object"
    }"#;
    fs::write(dir.path().join("root.json"), schema1).unwrap();
    fs::write(subdir.join("nested.json"), schema2).unwrap();

    let schemas = load_schemas_recursively(dir.path(), true).unwrap();
    assert!(schemas.contains_key("RootComponent"));
    assert!(schemas.contains_key("NestedComponent"));
}
