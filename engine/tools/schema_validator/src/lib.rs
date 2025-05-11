use serde_json::Value;

pub fn validate_schema(schema_json: &str) -> Result<(), String> {
    let value: Value =
        serde_json::from_str(schema_json).map_err(|e| format!("Invalid JSON: {e}"))?;

    // 1. Check for "title"
    if value.get("title").is_none() {
        return Err("Missing required field 'title'".to_string());
    }

    // 2. Check for "modes"
    let modes = match value.get("modes") {
        Some(m) => m,
        None => return Err("Missing required field 'modes'".to_string()),
    };

    // 3. Check that "modes" is an array of valid strings
    let allowed_modes = [
        "colony",
        "roguelike",
        "editor",
        "simulation",
        "single",
        "multi",
    ];
    if let Some(arr) = modes.as_array() {
        for mode in arr {
            let mode_str = mode.as_str().ok_or("Mode must be a string")?;
            if !allowed_modes.contains(&mode_str) {
                return Err(format!("Unknown mode '{}'", mode_str));
            }
        }
    } else {
        return Err("\"modes\" must be an array".to_string());
    }

    // 4. Check for min > max in properties
    if let Some(props) = value.get("properties").and_then(|p| p.as_object()) {
        for (prop_name, prop) in props {
            if let Some(min) = prop.get("minimum").and_then(|v| v.as_f64()) {
                if let Some(max) = prop.get("maximum").and_then(|v| v.as_f64()) {
                    if min > max {
                        return Err(format!("Property '{}' has minimum > maximum", prop_name));
                    }
                }
            }
        }
    }

    Ok(())
}
