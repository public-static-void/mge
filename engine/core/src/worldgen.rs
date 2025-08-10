use crate::map::deserialize::validate_map_schema;
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

/// Threadsafe plugin trait for global registry
pub trait ThreadSafeScriptingWorldgenPlugin: Send + Sync + DynClone {
    /// Invoke plugin
    fn invoke(&self, params: &JsonValue) -> Result<JsonValue, Box<dyn std::error::Error>>;
    /// Plugin backend
    fn backend(&self) -> &str;
}
dyn_clone::clone_trait_object!(ThreadSafeScriptingWorldgenPlugin);

/// Non-threadsafe plugin trait for local registries (Lua)
pub trait ScriptingWorldgenPlugin: DynClone {
    /// Invoke plugin
    fn invoke(&self, params: &JsonValue) -> Result<JsonValue, Box<dyn std::error::Error>>;
    /// Plugin backend
    fn backend(&self) -> &str;
}
dyn_clone::clone_trait_object!(ScriptingWorldgenPlugin);

/// Thread-safe plugin enum for global registry
pub enum ThreadSafeWorldgenPlugin {
    /// CAbi plugin
    CAbi {
        /// Plugin name
        name: String,
        /// World generate function
        generate: Arc<dyn Fn(&JsonValue) -> JsonValue + Send + Sync>,
        /// Library
        _lib: Option<Library>,
    },
    /// Thread-safe scripting plugin
    ThreadSafeScripting {
        /// Plugin name
        name: String,
        /// Plugin backend
        backend: String,
        /// Opaque type
        opaque: Box<dyn ThreadSafeScriptingWorldgenPlugin + Send + Sync>,
    },
}

/// Plugin enum for local registries (can include non-threadsafe scripting)
pub enum WorldgenPlugin {
    /// CAbi plugin
    CAbi {
        /// Plugin name
        name: String,
        /// World generate function
        generate: Arc<dyn Fn(&JsonValue) -> JsonValue + Send + Sync>,
        /// Library
        _lib: Option<Library>,
    },
    /// Thread-safe scripting plugin
    ThreadSafeScripting {
        /// Plugin name
        name: String,
        /// Plugin backend
        backend: String,
        /// Opaque type
        opaque: Box<dyn ThreadSafeScriptingWorldgenPlugin + Send + Sync>,
    },
    /// Non-threadsafe scripting plugin
    Scripting {
        /// Plugin name
        name: String,
        /// Plugin backend
        backend: String,
        /// Opaque type
        opaque: Box<dyn ScriptingWorldgenPlugin>,
    },
}

/// Worldgen errors
#[derive(Debug)]
pub enum WorldgenError {
    /// Plugin not found
    NotFound,
    /// Plugin error
    ScriptError(String),
    /// Validation error
    ValidationError(String),
}

impl fmt::Display for WorldgenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
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
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            validators: Vec::new(),
            postprocessors: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: ThreadSafeWorldgenPlugin) {
        self.plugins.push(plugin);
    }

    /// List plugin names
    pub fn list_names(&self) -> Vec<String> {
        self.plugins
            .iter()
            .map(|p| match p {
                ThreadSafeWorldgenPlugin::CAbi { name, .. } => name.clone(),
                ThreadSafeWorldgenPlugin::ThreadSafeScripting { name, .. } => name.clone(),
            })
            .collect()
    }

    /// Register a validator
    pub fn register_validator<F>(&mut self, f: F)
    where
        F: Fn(&serde_json::Value) -> Result<(), String> + Send + Sync + 'static,
    {
        self.validators.push(Box::new(f));
    }

    /// Register a postprocessor
    pub fn register_postprocessor<F>(&mut self, f: F)
    where
        F: Fn(&mut serde_json::Value) + Send + Sync + 'static,
    {
        self.postprocessors.push(Box::new(f));
    }

    /// Run validators
    pub fn run_validators(&self, map: &serde_json::Value) -> Result<(), String> {
        for validator in &self.validators {
            validator(map)?;
        }
        Ok(())
    }

    /// Run postprocessors
    pub fn run_postprocessors(&self, map: &mut serde_json::Value) {
        for post in &self.postprocessors {
            post(map);
        }
    }

    /// Invoke a plugin
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
                // --- NEW: Validate map schema here ---
                validate_map_schema(&map).map_err(WorldgenError::ValidationError)?;
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
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            validators: Vec::new(),
            postprocessors: Vec::new(),
            scripting_validators: Vec::new(),
            scripting_postprocessors: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: WorldgenPlugin) {
        self.plugins.push(plugin);
    }

    /// List plugin names
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

    /// Registers a validator
    pub fn register_validator<F>(&mut self, f: F)
    where
        F: Fn(&serde_json::Value) -> Result<(), String> + Send + Sync + 'static,
    {
        self.validators.push(Box::new(f));
    }

    /// Registers a postprocessor
    pub fn register_postprocessor<F>(&mut self, f: F)
    where
        F: Fn(&mut serde_json::Value) + Send + Sync + 'static,
    {
        self.postprocessors.push(Box::new(f));
    }

    /// Registers a validator for scripting
    pub fn register_scripting_validator<F>(&mut self, f: F)
    where
        F: Fn(&serde_json::Value) -> Result<(), String> + 'static,
    {
        self.scripting_validators.push(Box::new(f));
    }

    /// Registers a postprocessor for scripting
    pub fn register_scripting_postprocessor<F>(&mut self, f: F)
    where
        F: Fn(&mut serde_json::Value) + 'static,
    {
        self.scripting_postprocessors.push(Box::new(f));
    }

    /// Runs validators
    pub fn run_validators(&self, map: &serde_json::Value) -> Result<(), String> {
        for validator in &self.validators {
            validator(map)?;
        }
        for validator in &self.scripting_validators {
            validator(map)?;
        }
        Ok(())
    }

    /// Runs postprocessors
    pub fn run_postprocessors(&self, map: &mut serde_json::Value) {
        for post in &self.postprocessors {
            post(map);
        }
        for post in &self.scripting_postprocessors {
            post(map);
        }
    }

    /// Runs a plugin by name
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
                // --- NEW: Validate map schema here ---
                validate_map_schema(&map).map_err(WorldgenError::ValidationError)?;
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

/// Global thread-safe registry for world generation.
/// Only the thread-safe registry is global
pub static GLOBAL_WORLDGEN_REGISTRY: Lazy<Mutex<ThreadSafeWorldgenRegistry>> =
    Lazy::new(|| Mutex::new(ThreadSafeWorldgenRegistry::new()));
