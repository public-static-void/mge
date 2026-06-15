use serde::{Deserialize, Serialize};

/// Mod system
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    /// Mod name
    pub name: String,
    /// Mod version
    pub version: String,
    /// Mod description
    #[serde(default)]
    pub description: String,
    /// Main script file path
    pub main_script: Option<String>,
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

impl ModManifest {
    /// Validate the manifest, returning a list of errors.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.main_script.is_none() {
            errors.push("No main_script field found in mod manifest".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
