use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ModSystem {
    pub file: String,
    pub name: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub schemas: Vec<String>,
    #[serde(default)]
    pub systems: Vec<ModSystem>,
    #[serde(default)]
    pub scripts: Vec<String>,
}
