use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A component schema loaded from a JSON file.
#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSchema {
    pub name: String,
    pub schema: Value, // Store as serde_json::Value for maximum compatibility
    pub modes: Vec<String>,
}

/// Loads all component schemas from a directory.
///
/// Each file must be a `.json` file containing a valid JSON Schema.
/// The schema's name is taken from the "name" or "title" field, or the filename if missing.
/// The "modes" field is optional and should be an array of strings.
pub fn load_schemas_from_dir<P: AsRef<Path>>(
    dir: P,
) -> anyhow::Result<HashMap<String, ComponentSchema>> {
    let mut map = HashMap::new();
    let dir = dir.as_ref();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path: PathBuf = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let data = fs::read_to_string(&path)?;
            let json_val: Value = serde_json::from_str(&data)?;

            // Extract name: priority is "name" > "title" > filename
            let name = json_val
                .get("name")
                .and_then(|v| v.as_str())
                .or_else(|| json_val.get("title").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    path.file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "unknown".to_string())
                });

            // Extract modes as Vec<String>
            let modes = json_val
                .get("modes")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            // Store the whole schema as serde_json::Value for maximum flexibility
            map.insert(
                name.clone(),
                ComponentSchema {
                    name,
                    schema: json_val,
                    modes,
                },
            );
        }
    }

    Ok(map)
}
