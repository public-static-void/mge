use crate::plugins::types::{LoadedPlugin, PluginMetadata};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct PluginRegistry {
    plugins: RefCell<HashMap<String, Rc<LoadedPlugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RefCell::new(HashMap::new()),
        }
    }

    pub fn register(&self, plugin: LoadedPlugin) {
        let name = plugin.metadata.manifest.name.clone();
        self.plugins.borrow_mut().insert(name, Rc::new(plugin));
    }

    pub fn list(&self) -> Vec<String> {
        self.plugins.borrow().keys().cloned().collect()
    }

    pub fn get_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.plugins.borrow().get(name).map(|p| p.metadata.clone())
    }

    pub fn get_plugin(&self, name: &str) -> Option<Rc<LoadedPlugin>> {
        self.plugins.borrow().get(name).cloned()
    }

    pub fn all_metadata(&self) -> Vec<PluginMetadata> {
        self.plugins
            .borrow()
            .values()
            .map(|p| p.metadata.clone())
            .collect()
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
