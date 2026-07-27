//! Item definition registry for loading, caching, and looking up item definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

/// Registry for loading, caching, and looking up item definitions.
///
/// Items are loaded from JSON files. Duplicate IDs resolve by later
/// registration winning (mod overrides engine default).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemRegistry {
    items: HashMap<String, JsonValue>,
}

impl ItemRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load all `.json` item files from a directory.
    ///
    /// Duplicate IDs override previous registrations (mod overrides engine).
    /// Returns error if directory doesn't exist or contains invalid JSON.
    /// Each JSON file must contain an `id` field.
    pub fn load_items_from_dir(&mut self, dir: &Path) -> Result<(), String> {
        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("Failed to read item dir: {e}"))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
                let item: JsonValue = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
                let id = item.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                    format!("Item in {} missing required 'id' field", path.display())
                })?;
                self.items.insert(id.to_string(), item);
            }
        }
        Ok(())
    }

    /// Register a single item definition at runtime.
    ///
    /// Validates that the definition contains required fields: `id`, `name`, `slot`.
    /// Duplicate IDs override previous registration.
    pub fn register_item(&mut self, definition: JsonValue) -> Result<(), String> {
        let id = definition
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Item definition missing required 'id' field".to_string())?;

        let _name = definition
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Item '{id}' missing required 'name' field"))?;

        let _slot = definition
            .get("slot")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Item '{id}' missing required 'slot' field"))?;

        self.items.insert(id.to_string(), definition);
        Ok(())
    }

    /// Get an item definition by ID.
    pub fn get_item(&self, id: &str) -> Option<&JsonValue> {
        self.items.get(id)
    }

    /// List all registered item IDs.
    pub fn list_items(&self) -> Vec<String> {
        self.items.keys().cloned().collect()
    }
}
