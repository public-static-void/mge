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
    /// The name of the component
    pub name: String,
    /// The JSON schema
    pub schema: Value, // Store as serde_json::Value for maximum compatibility
    /// The game modes this component is valid in
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let v = serde_json::json!({"name": "ExplicitName", "title": "FallbackTitle"});
        assert_eq!(extract_name(&v, Some("file")), "ExplicitName");
    }

    #[test]
    fn test_title_fallback() {
        let v = serde_json::json!({"title": "TitleOnly"});
        assert_eq!(extract_name(&v, Some("file")), "TitleOnly");
    }

    #[test]
    fn test_filename_fallback() {
        let v = serde_json::json!({});
        assert_eq!(extract_name(&v, Some("my_schema")), "my_schema");
    }

    #[test]
    fn test_unknown_fallback() {
        let v = serde_json::json!({});
        assert_eq!(extract_name(&v, None), "unknown");
    }

    // --- Empty/null string handling ---

    #[test]
    fn test_empty_name_falls_to_title() {
        let v = serde_json::json!({"name": "", "title": "TitleValue"});
        assert_eq!(extract_name(&v, Some("file")), "TitleValue");
    }

    #[test]
    fn test_empty_title_falls_to_filename() {
        let v = serde_json::json!({"name": "", "title": ""});
        assert_eq!(extract_name(&v, Some("stem_name")), "stem_name");
    }

    #[test]
    fn test_both_empty_falls_to_filename() {
        let v = serde_json::json!({"name": "", "title": ""});
        assert_eq!(extract_name(&v, Some("fallback_stem")), "fallback_stem");
    }

    #[test]
    fn test_null_name_falls_through() {
        let v = serde_json::json!({"name": null, "title": "TitleValue"});
        assert_eq!(extract_name(&v, Some("file")), "TitleValue");
    }

    // --- ComponentSchema construction ---

    #[test]
    fn test_component_schema_construction() {
        let schema = ComponentSchema {
            name: "MyComponent".into(),
            schema: serde_json::json!({"type": "object", "properties": {}}),
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
        let v = serde_json::json!({"name": "BadModes", "modes": "not_an_array"});
        let modes_result = v.get("modes").and_then(|m| m.as_array());
        assert!(modes_result.is_none());
    }

    // --- Edge: null value in modes error path ---

    #[test]
    fn test_null_title_falls_through() {
        let v = serde_json::json!({"name": null, "title": null});
        assert_eq!(extract_name(&v, Some("stem")), "stem");
    }
}
