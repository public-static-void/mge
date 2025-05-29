use schemars::schema::RootSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentSchema {
    pub name: String,
    pub schema: RootSchema,
    pub modes: Vec<String>,
}

pub fn load_schemas_from_dir<P: AsRef<Path>>(
    dir: P,
) -> anyhow::Result<HashMap<String, ComponentSchema>> {
    let mut map = HashMap::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry
            .path()
            .extension()
            .map(|e| e == "json")
            .unwrap_or(false)
        {
            let data = fs::read_to_string(entry.path())?;
            let json_val: serde_json::Value = serde_json::from_str(&data)?;

            // Get name from "title"
            let name = json_val
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| entry.path().file_stem().unwrap().to_string_lossy().into());

            // Get modes from root-level "modes"
            let modes = json_val
                .get("modes")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            // Parse the whole file as a RootSchema
            let schema: RootSchema = serde_json::from_str(&data)?;

            map.insert(
                name.clone(),
                ComponentSchema {
                    name,
                    schema,
                    modes,
                },
            );
        }
    }
    Ok(map)
}
