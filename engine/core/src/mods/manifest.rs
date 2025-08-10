use serde::Deserialize;

/// Mod system
#[derive(Debug, Deserialize)]
pub struct ModSystem {
    /// Mod file
    pub file: String,
    /// Mod name
    pub name: String,
    /// Mod dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Mod manifest
#[derive(Debug, Deserialize)]
pub struct ModManifest {
    /// Mod name
    pub name: String,
    /// Mod version
    pub version: String,
    /// Mod description
    #[serde(default)]
    pub description: String,
    /// Mod dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Mod schemas
    #[serde(default)]
    pub schemas: Vec<String>,
    /// Mod systems
    #[serde(default)]
    pub systems: Vec<ModSystem>,
    /// Mod scripts
    #[serde(default)]
    pub scripts: Vec<String>,
}
