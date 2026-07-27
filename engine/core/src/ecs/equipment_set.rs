//! Equipment set registry for loading, caching, and looking up named loadout blueprints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A named loadout blueprint mapping slot names to item IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentSet {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// Mapping of slot names to item IDs.
    pub items: HashMap<String, String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// Registry for loading, caching, and looking up equipment sets.
///
/// Sets are loaded from JSON files. Duplicate names resolve by later
/// registration winning (mod overrides engine default).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EquipmentSetRegistry {
    sets: HashMap<String, EquipmentSet>,
}

impl EquipmentSetRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load all `.json` equipment set files from a directory.
    ///
    /// Duplicate names override previous registrations (mod overrides engine).
    /// Returns error if directory doesn't exist or contains invalid JSON.
    /// Each JSON file must contain a `name` field.
    pub fn load_sets_from_dir(&mut self, dir: &Path) -> Result<(), String> {
        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("Failed to read equipment set dir: {e}"))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
                let set: EquipmentSet = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
                self.sets.insert(set.name.clone(), set);
            }
        }
        Ok(())
    }

    /// Register a single equipment set at runtime.
    ///
    /// Duplicate names override previous registration.
    pub fn register_set(&mut self, set: EquipmentSet) {
        self.sets.insert(set.name.clone(), set);
    }

    /// Get an equipment set by name.
    pub fn get_set(&self, name: &str) -> Option<&EquipmentSet> {
        self.sets.get(name)
    }

    /// List all registered equipment set names.
    pub fn list_sets(&self) -> Vec<String> {
        self.sets.keys().cloned().collect()
    }
}
