use serde_json::Value;
use std::collections::HashMap;
use std::fs;

lazy_static::lazy_static! {
    pub static ref SCHEMA_REGISTRY: HashMap<String, Value> = {
        let mut map = HashMap::new();
        let schema_dir = "engine/assets/schemas";
        if let Ok(entries) = fs::read_dir(schema_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(schema_str) = fs::read_to_string(&path) {
                            if let Ok(schema) = serde_json::from_str::<Value>(&schema_str) {
                                // Normalize key to lowercase for case-insensitive lookup
                                map.insert(stem.to_ascii_lowercase(), schema);
                            } else {
                                eprintln!("Failed to parse schema: {path:?}");
                            }
                        } else {
                            eprintln!("Failed to read schema file: {path:?}");
                        }
                    }
            }
        }
        map
    };
}

/// Helper to get a schema by name, case-insensitive.
pub fn get_schema(name: &str) -> Option<&Value> {
    SCHEMA_REGISTRY.get(&name.to_ascii_lowercase())
}
