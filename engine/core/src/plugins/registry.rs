use crate::plugins::types::PluginMetadata;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct PluginRegistry {
    plugins: RefCell<HashMap<String, PluginMetadata>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RefCell::new(HashMap::new()),
        }
    }

    pub fn register(&self, metadata: PluginMetadata) {
        let name = metadata.manifest.name.clone();
        self.plugins.borrow_mut().insert(name, metadata);
    }

    pub fn list(&self) -> Vec<String> {
        self.plugins.borrow().keys().cloned().collect()
    }

    pub fn get_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.plugins.borrow().get(name).cloned()
    }

    pub fn all_metadata(&self) -> Vec<PluginMetadata> {
        self.plugins.borrow().values().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.plugins.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.borrow().is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
