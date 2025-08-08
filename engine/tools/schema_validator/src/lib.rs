use serde_json::Value;

/// Validates that the JSON is a schema with a title and no invalid property constraints.
/// Validates that all modes are in the provided allowed_modes list.
pub fn validate_schema(schema_json: &str, allowed_modes: &[String]) -> Result<(), String> {
    let value: Value =
        serde_json::from_str(schema_json).map_err(|e| format!("Invalid JSON: {e}"))?;

    // 1. Check for "title" (all schemas should have a title for identification)
    if value.get("title").is_none() {
        return Err("Missing required field 'title'".to_string());
    }

    // 2. Check for min > max in properties (generic data sanity)
    if let Some(props) = value.get("properties").and_then(|p| p.as_object()) {
        for (prop_name, prop) in props {
            if let Some(min) = prop.get("minimum").and_then(|v| v.as_f64())
                && let Some(max) = prop.get("maximum").and_then(|v| v.as_f64())
                && min > max
            {
                return Err(format!("Property '{prop_name}' has minimum > maximum"));
            }
        }
    }

    // 3. Check for invalid modes if present
    if let Some(modes) = value.get("modes").and_then(|m| m.as_array()) {
        for mode in modes {
            if let Some(mode_str) = mode.as_str()
                && !allowed_modes.iter().any(|m| m == mode_str)
            {
                return Err(format!("Unknown mode '{mode_str}'"));
            }
        }
    }

    Ok(())
}
