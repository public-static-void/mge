use libloading::Library;
use once_cell::sync::Lazy;
use serde_json::Value as JsonValue;
use std::fmt;
use std::sync::{Arc, Mutex};

// --- dyn-clone for trait object cloning ---
use dyn_clone::DynClone;

// Type aliases for thread-safe hooks
type WorldgenValidator = Box<dyn Fn(&serde_json::Value) -> Result<(), String> + Send + Sync>;
type WorldgenPostprocessor = Box<dyn Fn(&mut serde_json::Value) + Send + Sync>;

// Type aliases for non-thread-safe scripting hooks
type ScriptingValidator = Box<dyn Fn(&serde_json::Value) -> Result<(), String>>;
type ScriptingPostprocessor = Box<dyn Fn(&mut serde_json::Value)>;

// Threadsafe plugin trait for global registry
pub trait ThreadSafeScriptingWorldgenPlugin: Send + Sync + DynClone {
    fn invoke(&self, params: &JsonValue) -> Result<JsonValue, Box<dyn std::error::Error>>;
    fn backend(&self) -> &str;
}
dyn_clone::clone_trait_object!(ThreadSafeScriptingWorldgenPlugin);

// Non-threadsafe plugin trait for local registries (Lua)
pub trait ScriptingWorldgenPlugin: DynClone {
    fn invoke(&self, params: &JsonValue) -> Result<JsonValue, Box<dyn std::error::Error>>;
    fn backend(&self) -> &str;
}
dyn_clone::clone_trait_object!(ScriptingWorldgenPlugin);

// THREAD-SAFE plugin enum for global registry
pub enum ThreadSafeWorldgenPlugin {
    CAbi {
        name: String,
        generate: Arc<dyn Fn(&JsonValue) -> JsonValue + Send + Sync>,
        _lib: Option<Library>,
    },
    ThreadSafeScripting {
        name: String,
        backend: String,
        opaque: Box<dyn ThreadSafeScriptingWorldgenPlugin + Send + Sync>,
    },
}

// FULL plugin enum for local registries (can include non-threadsafe scripting)
pub enum WorldgenPlugin {
    CAbi {
        name: String,
        generate: Arc<dyn Fn(&JsonValue) -> JsonValue + Send + Sync>,
        _lib: Option<Library>,
    },
    ThreadSafeScripting {
        name: String,
        backend: String,
        opaque: Box<dyn ThreadSafeScriptingWorldgenPlugin + Send + Sync>,
    },
    Scripting {
        name: String,
        backend: String,
        opaque: Box<dyn ScriptingWorldgenPlugin>,
    },
}

#[derive(Debug)]
pub enum WorldgenError {
    NotFound,
    ScriptError(String),
    ValidationError(String),
}

impl fmt::Display for WorldgenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for WorldgenError {}

/// Thread-safe global registry: only thread-safe plugins and hooks!
pub struct ThreadSafeWorldgenRegistry {
    pub(crate) plugins: Vec<ThreadSafeWorldgenPlugin>,
    validators: Vec<WorldgenValidator>,
    postprocessors: Vec<WorldgenPostprocessor>,
}

impl ThreadSafeWorldgenRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            validators: Vec::new(),
            postprocessors: Vec::new(),
        }
    }

    pub fn register(&mut self, plugin: ThreadSafeWorldgenPlugin) {
        self.plugins.push(plugin);
    }

    pub fn list_names(&self) -> Vec<String> {
        self.plugins
            .iter()
            .map(|p| match p {
                ThreadSafeWorldgenPlugin::CAbi { name, .. } => name.clone(),
                ThreadSafeWorldgenPlugin::ThreadSafeScripting { name, .. } => name.clone(),
            })
            .collect()
    }

    pub fn register_validator<F>(&mut self, f: F)
    where
        F: Fn(&serde_json::Value) -> Result<(), String> + Send + Sync + 'static,
    {
        self.validators.push(Box::new(f));
    }

    pub fn register_postprocessor<F>(&mut self, f: F)
    where
        F: Fn(&mut serde_json::Value) + Send + Sync + 'static,
    {
        self.postprocessors.push(Box::new(f));
    }

    pub fn run_validators(&self, map: &serde_json::Value) -> Result<(), String> {
        for validator in &self.validators {
            validator(map)?;
        }
        Ok(())
    }

    pub fn run_postprocessors(&self, map: &mut serde_json::Value) {
        for post in &self.postprocessors {
            post(map);
        }
    }

    pub fn invoke(&self, name: &str, params: &JsonValue) -> Result<JsonValue, WorldgenError> {
        for plugin in &self.plugins {
            let plugin_name = match plugin {
                ThreadSafeWorldgenPlugin::CAbi { name, .. } => name,
                ThreadSafeWorldgenPlugin::ThreadSafeScripting { name, .. } => name,
            };
            if plugin_name == name {
                let mut map = match plugin {
                    ThreadSafeWorldgenPlugin::CAbi { generate, .. } => generate(params),
                    ThreadSafeWorldgenPlugin::ThreadSafeScripting { opaque, .. } => opaque
                        .invoke(params)
                        .map_err(|e| WorldgenError::ScriptError(e.to_string()))?,
                };
                self.run_postprocessors(&mut map);
                self.run_validators(&map)
                    .map_err(WorldgenError::ValidationError)?;
                return Ok(map);
            }
        }
        Err(WorldgenError::NotFound)
    }
}

impl Default for ThreadSafeWorldgenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Local (non-thread-safe) registry: can store Lua plugins and non-thread-safe hooks
pub struct WorldgenRegistry {
    pub(crate) plugins: Vec<WorldgenPlugin>,
    validators: Vec<WorldgenValidator>,
    postprocessors: Vec<WorldgenPostprocessor>,
    scripting_validators: Vec<ScriptingValidator>,
    scripting_postprocessors: Vec<ScriptingPostprocessor>,
}

impl WorldgenRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            validators: Vec::new(),
            postprocessors: Vec::new(),
            scripting_validators: Vec::new(),
            scripting_postprocessors: Vec::new(),
        }
    }

    pub fn register(&mut self, plugin: WorldgenPlugin) {
        self.plugins.push(plugin);
    }

    pub fn list_names(&self) -> Vec<String> {
        self.plugins
            .iter()
            .map(|p| match p {
                WorldgenPlugin::CAbi { name, .. } => name.clone(),
                WorldgenPlugin::ThreadSafeScripting { name, .. } => name.clone(),
                WorldgenPlugin::Scripting { name, .. } => name.clone(),
            })
            .collect()
    }

    pub fn register_validator<F>(&mut self, f: F)
    where
        F: Fn(&serde_json::Value) -> Result<(), String> + Send + Sync + 'static,
    {
        self.validators.push(Box::new(f));
    }

    pub fn register_postprocessor<F>(&mut self, f: F)
    where
        F: Fn(&mut serde_json::Value) + Send + Sync + 'static,
    {
        self.postprocessors.push(Box::new(f));
    }

    pub fn register_scripting_validator<F>(&mut self, f: F)
    where
        F: Fn(&serde_json::Value) -> Result<(), String> + 'static,
    {
        self.scripting_validators.push(Box::new(f));
    }

    pub fn register_scripting_postprocessor<F>(&mut self, f: F)
    where
        F: Fn(&mut serde_json::Value) + 'static,
    {
        self.scripting_postprocessors.push(Box::new(f));
    }

    pub fn run_validators(&self, map: &serde_json::Value) -> Result<(), String> {
        for validator in &self.validators {
            validator(map)?;
        }
        for validator in &self.scripting_validators {
            validator(map)?;
        }
        Ok(())
    }

    pub fn run_postprocessors(&self, map: &mut serde_json::Value) {
        for post in &self.postprocessors {
            post(map);
        }
        for post in &self.scripting_postprocessors {
            post(map);
        }
    }

    pub fn invoke(&self, name: &str, params: &JsonValue) -> Result<JsonValue, WorldgenError> {
        for plugin in &self.plugins {
            let plugin_name = match plugin {
                WorldgenPlugin::CAbi { name, .. } => name,
                WorldgenPlugin::ThreadSafeScripting { name, .. } => name,
                WorldgenPlugin::Scripting { name, .. } => name,
            };
            if plugin_name == name {
                let mut map = match plugin {
                    WorldgenPlugin::CAbi { generate, .. } => generate(params),
                    WorldgenPlugin::ThreadSafeScripting { opaque, .. } => opaque
                        .invoke(params)
                        .map_err(|e| WorldgenError::ScriptError(e.to_string()))?,
                    WorldgenPlugin::Scripting { opaque, .. } => opaque
                        .invoke(params)
                        .map_err(|e| WorldgenError::ScriptError(e.to_string()))?,
                };
                self.run_postprocessors(&mut map);
                self.run_validators(&map)
                    .map_err(WorldgenError::ValidationError)?;
                return Ok(map);
            }
        }
        Err(WorldgenError::NotFound)
    }

    /// Copies all CAbi and ThreadSafeScripting plugins from a ThreadSafeWorldgenRegistry.
    pub fn import_threadsafe_plugins(&mut self, global: &ThreadSafeWorldgenRegistry) {
        for plugin in &global.plugins {
            match plugin {
                ThreadSafeWorldgenPlugin::CAbi {
                    name,
                    generate,
                    _lib: _,
                } => {
                    self.plugins.push(WorldgenPlugin::CAbi {
                        name: name.clone(),
                        generate: Arc::clone(generate),
                        _lib: None, // Do not clone Library, only global registry holds it
                    });
                }
                ThreadSafeWorldgenPlugin::ThreadSafeScripting {
                    name,
                    backend,
                    opaque,
                } => {
                    self.plugins.push(WorldgenPlugin::ThreadSafeScripting {
                        name: name.clone(),
                        backend: backend.clone(),
                        opaque: dyn_clone::clone_box(&**opaque),
                    });
                }
            }
        }
    }
}

impl Default for WorldgenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Only the thread-safe registry is global
pub static GLOBAL_WORLDGEN_REGISTRY: Lazy<Mutex<ThreadSafeWorldgenRegistry>> =
    Lazy::new(|| Mutex::new(ThreadSafeWorldgenRegistry::new()));
