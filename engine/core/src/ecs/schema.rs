use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A component schema loaded from a JSON file.
/// - `name`: The unique name of the component.
/// - `schema`: The full JSON schema as a serde_json::Value.
/// - `modes`: List of game modes this component is valid in.
#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSchema {
    pub name: String,
    pub schema: Value, // Store as serde_json::Value for maximum compatibility
    pub modes: Vec<String>,
}

/// Loads all component schemas from a directory, validating them.
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
            schema_validator::validate_schema(&data)
                .map_err(|e| anyhow::anyhow!("{}: {}", path.display(), e))?;
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

/// Save a component schema to a file as pretty-printed JSON.
pub fn save_schema_to_file(schema: &ComponentSchema, path: &Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(&schema.schema)?;
    fs::write(path, json)?;
    Ok(())
}

/// Loads all component schemas from a directory and all subdirectories, optionally validating them.
pub fn load_schemas_recursively<P: AsRef<Path>>(
    dir: P,
    validate: bool,
) -> anyhow::Result<HashMap<String, ComponentSchema>> {
    let mut map = HashMap::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
            let data = fs::read_to_string(path)?;
            if validate {
                schema_validator::validate_schema(&data)
                    .map_err(|e| anyhow::anyhow!("{}: {}", path.display(), e))?;
            }
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
