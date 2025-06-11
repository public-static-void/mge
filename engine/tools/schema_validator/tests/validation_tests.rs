use schema_validator::validate_schema;

fn allowed_modes() -> Vec<String> {
    vec![
        "colony".to_string(),
        "roguelike".to_string(),
        "editor".to_string(),
        "simulation".to_string(),
        "single".to_string(),
        "multi".to_string(),
    ]
}

#[test]
fn test_missing_title_field() {
    let schema_json = r#"
    {
        "type": "object",
        "properties": {
            "value": { "type": "number" }
        },
        "required": ["value"],
        "modes": ["colony"]
    }
    "#;

    let result = validate_schema(schema_json, &allowed_modes());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("Missing required field 'title'")
    );
}

#[test]
fn test_invalid_mode_name() {
    let schema_json = r#"
    {
        "title": "Mana",
        "type": "object",
        "properties": {
            "value": { "type": "number" }
        },
        "required": ["value"],
        "modes": ["invalid_mode"]
    }
    "#;

    let result = validate_schema(schema_json, &allowed_modes());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown mode 'invalid_mode'"));
}

#[test]
fn test_min_greater_than_max() {
    let schema_json = r#"
    {
        "title": "Health",
        "type": "object",
        "properties": {
            "current": { "type": "number", "minimum": 100, "maximum": 50 }
        },
        "required": ["current"],
        "modes": ["colony"]
    }
    "#;

    let result = validate_schema(schema_json, &allowed_modes());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("Property 'current' has minimum > maximum")
    );
}
