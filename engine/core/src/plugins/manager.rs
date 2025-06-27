use crate::plugins::subprocess::{PluginRequest, PluginResponse, PluginSubprocess};
use std::collections::HashMap;
use std::path::Path;

pub struct PluginManager {
    plugins: HashMap<String, PluginSubprocess>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn launch_plugin<P: AsRef<Path>>(
        &mut self,
        name: String,
        bin_path: P,
        socket_path: &str,
    ) -> Result<(), String> {
        if self.plugins.contains_key(&name) {
            return Err(format!("Plugin '{name}' already running"));
        }
        let subprocess = PluginSubprocess::spawn(bin_path, socket_path)?;
        self.plugins.insert(name, subprocess);
        Ok(())
    }

    pub fn send(&mut self, name: &str, request: &PluginRequest) -> Result<PluginResponse, String> {
        let plugin = self.plugins.get_mut(name).ok_or("Plugin not found")?;
        plugin.send_request(request)
    }

    pub fn reload_plugin<P: AsRef<Path>>(
        &mut self,
        name: &str,
        bin_path: P,
        socket_path: &str,
    ) -> Result<(), String> {
        self.shutdown_plugin(name)?;
        self.launch_plugin(name.to_string(), bin_path, socket_path)
    }

    pub fn shutdown_plugin(&mut self, name: &str) -> Result<(), String> {
        if let Some(mut plugin) = self.plugins.remove(name) {
            plugin.send_request(&PluginRequest::Shutdown).ok();
            plugin.terminate();
        }
        Ok(())
    }

    pub fn shutdown_all(&mut self) {
        for (_name, mut plugin) in self.plugins.drain() {
            plugin.send_request(&PluginRequest::Shutdown).ok();
            plugin.terminate();
        }
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        self.shutdown_all();
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
