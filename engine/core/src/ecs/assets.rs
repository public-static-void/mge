//! Asset/data loader for the Modular Game Engine.
//!
//! Loads runtime data assets (resources, recipes, jobs, prototypes, etc.)
//! as opposed to ECS component schemas.

use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Loads all JSON files in a directory, indexed by a key field (e.g., "kind", "name", or "id").
///
/// # Arguments
/// * `dir` - The directory to search for JSON files.
/// * `key_field` - The JSON field to use as the map key (e.g., "kind" for resources).
///
/// # Returns
/// A map from key field to the parsed JSON value for each file.
pub fn load_json_assets_by_key<P: AsRef<Path>>(
    dir: P,
    key_field: &str,
) -> anyhow::Result<HashMap<String, Value>> {
    let mut map = HashMap::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                let data = fs::read_to_string(&path)?;
                let json_val: Value = serde_json::from_str(&data)?;
                if let Some(key) = json_val.get(key_field).and_then(|v| v.as_str()) {
                    map.insert(key.to_string(), json_val);
                }
            }
        }
    }
    Ok(map)
}

/// Loads all resource kind definitions (expects "kind" as key).
///
/// # Arguments
/// * `dir` - Directory containing resource kind JSON files.
///
/// # Returns
/// A map from resource kind to its definition.
pub fn load_resource_definitions<P: AsRef<Path>>(dir: P) -> anyhow::Result<HashMap<String, Value>> {
    load_json_assets_by_key(dir, "kind")
}

/// Loads all recipes (expects "name" as key).
///
/// # Arguments
/// * `dir` - Directory containing recipe JSON files.
///
/// # Returns
/// A map from recipe name to its definition.
pub fn load_recipes<P: AsRef<Path>>(dir: P) -> anyhow::Result<HashMap<String, Value>> {
    load_json_assets_by_key(dir, "name")
}

/// Loads all jobs (expects "name" as key).
///
/// # Arguments
/// * `dir` - Directory containing job JSON files.
///
/// # Returns
/// A map from job name to its definition.
pub fn load_jobs<P: AsRef<Path>>(dir: P) -> anyhow::Result<HashMap<String, Value>> {
    load_json_assets_by_key(dir, "name")
}

/// Loads all prototypes (expects "name" as key).
///
/// # Arguments
/// * `dir` - Directory containing prototype JSON files.
///
/// # Returns
/// A map from prototype name to its definition.
pub fn load_prototypes<P: AsRef<Path>>(dir: P) -> anyhow::Result<HashMap<String, Value>> {
    load_json_assets_by_key(dir, "name")
}
