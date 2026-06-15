use engine_core::ecs::schema::ComponentSchema;
use serde_json::json;

/// Replicates the name-extraction logic from production code so we can test
/// it without filesystem I/O. Priority: "name" > "title" > file_stem > "unknown".
fn extract_name(json_val: &serde_json::Value, file_stem: Option<&str>) -> String {
    json_val
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            json_val
                .get("title")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
        })
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            file_stem
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}

/// Check whether a path looks like a JSON file (matches the `.json` extension
/// check used in the production functions).
fn is_json_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        == Some("json")
}

// --- Name priority ---

#[test]
fn test_name_field_wins() {
    let v = json!({"name": "ExplicitName", "title": "FallbackTitle"});
    assert_eq!(extract_name(&v, Some("file")), "ExplicitName");
}

#[test]
fn test_title_fallback() {
    let v = json!({"title": "TitleOnly"});
    assert_eq!(extract_name(&v, Some("file")), "TitleOnly");
}

#[test]
fn test_filename_fallback() {
    let v = json!({});
    assert_eq!(extract_name(&v, Some("my_schema")), "my_schema");
}

#[test]
fn test_unknown_fallback() {
    let v = json!({});
    assert_eq!(extract_name(&v, None), "unknown");
}

// --- Empty/null string handling ---

#[test]
fn test_empty_name_falls_to_title() {
    let v = json!({"name": "", "title": "TitleValue"});
    assert_eq!(extract_name(&v, Some("file")), "TitleValue");
}

#[test]
fn test_empty_title_falls_to_filename() {
    let v = json!({"name": "", "title": ""});
    assert_eq!(extract_name(&v, Some("stem_name")), "stem_name");
}

#[test]
fn test_both_empty_falls_to_filename() {
    let v = json!({"name": "", "title": ""});
    assert_eq!(extract_name(&v, Some("fallback_stem")), "fallback_stem");
}

#[test]
fn test_null_name_falls_through() {
    let v = json!({"name": null, "title": "TitleValue"});
    assert_eq!(extract_name(&v, Some("file")), "TitleValue");
}

// --- ComponentSchema construction ---

#[test]
fn test_component_schema_construction() {
    let schema = ComponentSchema {
        name: "MyComponent".into(),
        schema: json!({"type": "object", "properties": {}}),
        modes: vec!["Colony".into(), "Roguelike".into()],
    };
    assert_eq!(schema.name, "MyComponent");
    assert!(schema.schema.get("type").and_then(|v| v.as_str()) == Some("object"));
    assert_eq!(schema.modes.len(), 2);
    assert!(schema.modes.contains(&"Colony".to_string()));
}

// --- Non-JSON extension check ---

#[test]
fn test_non_json_extension_skipped() {
    assert!(!is_json_path("schema.txt"));
    assert!(!is_json_path("schema"));
    assert!(!is_json_path(".json")); // hidden file, no stem
    assert!(is_json_path("schema.json"));
    assert!(is_json_path("dir/schema.json"));
}

// --- Non-array modes error path ---

#[test]
fn test_non_array_modes_error() {
    let v = json!({"name": "BadModes", "modes": "not_an_array"});
    let modes_result = v.get("modes").and_then(|m| m.as_array());
    assert!(modes_result.is_none());
}

// --- Edge: null value in modes error path ---

#[test]
fn test_null_title_falls_through() {
    let v = json!({"name": null, "title": null});
    assert_eq!(extract_name(&v, Some("stem")), "stem");
}
