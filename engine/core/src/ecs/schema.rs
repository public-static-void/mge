use schemars::schema::RootSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
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
            let schema: RootSchema = serde_json::from_str(&data)?;
            // Extract component name and modes from schema or file name
            let name = schema
                .schema
                .metadata
                .as_ref()
                .and_then(|m| m.title.clone())
                .unwrap_or_else(|| entry.path().file_stem().unwrap().to_string_lossy().into());
            let modes = schema
                .schema
                .extensions
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
                    schema,
                    modes,
                },
            );
        }
    }
    Ok(map)
}
