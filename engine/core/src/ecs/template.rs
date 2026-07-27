//! Unit template registry for loading, caching, and looking up unit templates.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

/// A unit template defining default components for an entity archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTemplate {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub components: HashMap<String, JsonValue>,
}

/// Registry for loading, caching, and looking up unit templates.
///
/// Templates are loaded from JSON files. Duplicate names resolve by later
/// registration winning (mod overrides engine default).
#[derive(Debug, Clone, Default)]
pub struct UnitTemplateRegistry {
    templates: HashMap<String, UnitTemplate>,
}

impl UnitTemplateRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load all `.json` template files from a directory.
    ///
    /// Duplicate names override previous registrations (mod overrides engine).
    /// Returns error if directory doesn't exist or contains invalid JSON.
    pub fn load_templates_from_dir(&mut self, dir: &Path) -> Result<(), String> {
        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("Failed to read template dir: {e}"))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
                let template: UnitTemplate = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
                self.templates.insert(template.name.clone(), template);
            }
        }
        Ok(())
    }

    /// Register a single template at runtime.
    ///
    /// Duplicate names override previous registration.
    pub fn register_template(&mut self, template: UnitTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Get a template by name.
    pub fn get_template(&self, name: &str) -> Option<&UnitTemplate> {
        self.templates.get(name)
    }

    /// List all registered template names.
    pub fn list_templates(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }
}
