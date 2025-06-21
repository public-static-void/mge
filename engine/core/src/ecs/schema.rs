//! ECS component schema loader and validator for the Modular Game Engine.
//!
//! Handles loading, validating, and registering JSON schemas for ECS components.

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A component schema loaded from a JSON file.
///
/// - `name`: The unique name of the component.
/// - `schema`: The full JSON schema as a serde_json::Value.
/// - `modes`: List of game modes this component is valid in.
#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSchema {
    pub name: String,
    pub schema: Value, // Store as serde_json::Value for maximum compatibility
    pub modes: Vec<String>,
}

/// Save a component schema to a file as pretty-printed JSON.
pub fn save_schema_to_file(schema: &ComponentSchema, path: &Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(&schema.schema)?;
    fs::write(path, json)?;
    Ok(())
}

/// Loads all component schemas from a directory and all subdirectories, optionally validating them.
///
/// # Arguments
/// * `dir` - The directory to search for JSON schema files.
/// * `validate` - Whether to validate schemas.
/// * `allowed_modes` - List of allowed game modes.
///
/// # Returns
/// A map from component name to its schema.
pub fn load_schemas_recursively<P: AsRef<Path>>(
    dir: P,
    validate: bool,
    allowed_modes: &[String],
) -> anyhow::Result<HashMap<String, ComponentSchema>> {
    let mut map = HashMap::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
            let data = fs::read_to_string(path)?;
            if validate {
                schema_validator::validate_schema(&data, allowed_modes)
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

/// Loads all component schemas from a directory, validating them,
/// and enforces allowed modes for ECS/gameplay components.
///
/// # Arguments
/// * `dir` - The directory to search for JSON schema files.
/// * `allowed_modes` - List of allowed game modes.
///
/// # Returns
/// A map from component name to its schema.
pub fn load_schemas_from_dir_with_modes<P: AsRef<Path>>(
    dir: P,
    allowed_modes: &[String],
) -> anyhow::Result<HashMap<String, ComponentSchema>> {
    let mut map = HashMap::new();
    let dir = dir.as_ref();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path: PathBuf = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let data = fs::read_to_string(&path)?;
            schema_validator::validate_schema(&data, allowed_modes)
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

            // ECS/gameplay component: must have "modes"
            // Only enforce modes if "modes" is present
            if let Some(modes_val) = json_val.get("modes") {
                let modes = modes_val.as_array().ok_or_else(|| {
                    anyhow::anyhow!("Component schema '{}' has non-array 'modes'", name)
                })?;
                for mode in modes {
                    let mode_str = mode.as_str().ok_or_else(|| {
                        anyhow::anyhow!("Mode in component '{}' must be a string", name)
                    })?;
                    if !allowed_modes.contains(&mode_str.to_string()) {
                        return Err(anyhow::anyhow!(
                            "Unknown mode '{}' in component '{}'",
                            mode_str,
                            name
                        ));
                    }
                }
            }

            // Extract modes as Vec<String> for all schemas (empty if not present)
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

/// Loads the allowed modes from the game configuration file (e.g., game.toml).
pub fn load_allowed_modes() -> anyhow::Result<Vec<String>> {
    use crate::config::GameConfig;
    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml");
    let config = GameConfig::load_from_file(config_path)?;
    Ok(config.allowed_modes)
}
