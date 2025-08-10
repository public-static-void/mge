use crate::plugins::types::PluginMetadata;
use std::cell::RefCell;
use std::collections::HashMap;

/// A registry for plugins
pub struct PluginRegistry {
    plugins: RefCell<HashMap<String, PluginMetadata>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: RefCell::new(HashMap::new()),
        }
    }

    /// Register a plugin
    pub fn register(&self, metadata: PluginMetadata) {
        let name = metadata.manifest.name.clone();
        self.plugins.borrow_mut().insert(name, metadata);
    }

    /// List all plugins
    pub fn list(&self) -> Vec<String> {
        self.plugins.borrow().keys().cloned().collect()
    }

    /// Get the metadata for a plugin
    pub fn get_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.plugins.borrow().get(name).cloned()
    }

    /// Get the metadata for all plugins
    pub fn all_metadata(&self) -> Vec<PluginMetadata> {
        self.plugins.borrow().values().cloned().collect()
    }

    /// Get the number of plugins
    pub fn len(&self) -> usize {
        self.plugins.borrow().len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.plugins.borrow().is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
