use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    pub native: Vec<String>,
    // Optionally: pub scripting: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameConfig {
    pub title: String,
    pub version: String,
    pub allowed_modes: Vec<String>,
    pub plugins: Option<PluginConfig>,
    // Add more fields as needed
}

impl GameConfig {
    /// Load the game config from a TOML file at the given path.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: GameConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
