use serde::Deserialize;
use std::fs;
use std::path::Path;

/// The plugin config file
#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    /// List of native plugins
    pub native: Vec<String>,
    // Optionally: pub scripting: Vec<String>,
}

/// The game config file
#[derive(Debug, Clone, Deserialize)]
pub struct GameConfig {
    /// Game title
    pub title: String,
    /// Game version
    pub version: String,
    /// Allowed game modes
    pub allowed_modes: Vec<String>,
    /// Game plugins
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
