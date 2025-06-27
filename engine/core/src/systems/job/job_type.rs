//! Job type definitions, logic registry, and data structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Data describing a job type, as loaded from data files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct JobTypeData {
    /// Name of the job type.
    pub name: String,
    /// List of requirement strings for this job type.
    #[serde(default)]
    pub requirements: Vec<String>,
    /// Optional duration for the job type.
    #[serde(default)]
    pub duration: Option<f64>,
    /// List of effects associated with this job type.
    #[serde(default)]
    pub effects: Vec<serde_json::Value>,
}

/// Effect definition for a job type.
/// (Kept for compatibility, but not used for serialization/deserialization anymore.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobEffect {
    /// Name of the action this effect performs.
    pub action: String,
    /// Optional 'from' field for effect source.
    #[serde(default)]
    pub from: Option<String>,
    /// Optional 'to' field for effect target.
    #[serde(default)]
    pub to: Option<String>,
}

/// Enum for native or scripted job logic.
#[derive(Clone, Debug)]
pub enum JobLogicKind {
    /// Native Rust function handler.
    Native(fn(&mut crate::ecs::world::World, u32, u32, &serde_json::Value) -> serde_json::Value),
    /// Scripted handler, identified by a unique key (opaque to core).
    Scripted(String),
}

/// Central registry for job types and their logic.
#[derive(Default)]
pub struct JobTypeRegistry {
    data: HashMap<String, JobTypeData>,
    logic: HashMap<String, JobLogicKind>,
}

impl JobTypeRegistry {
    /// Creates a new, empty job type registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the job type data for the given name, if present.
    pub fn get_data(&self, name: &str) -> Option<&JobTypeData> {
        self.data.get(name)
    }

    /// Returns a reference to the job logic for the given name, if present.
    pub fn get_logic(&self, name: &str) -> Option<&JobLogicKind> {
        self.logic.get(name)
    }

    /// Registers a job type and its logic.
    pub fn register(&mut self, data: JobTypeData, logic: JobLogicKind) {
        self.logic.insert(data.name.clone(), logic);
        self.data.insert(data.name.clone(), data);
    }

    /// Registers a native handler for a job type by name.
    pub fn register_native(
        &mut self,
        name: &str,
        handler: fn(
            &mut crate::ecs::world::World,
            u32,
            u32,
            &serde_json::Value,
        ) -> serde_json::Value,
    ) {
        let data = JobTypeData {
            name: name.to_string(),
            ..Default::default()
        };
        self.register(data, JobLogicKind::Native(handler))
    }

    /// Returns a vector of all registered job type names.
    pub fn job_type_names(&self) -> Vec<&str> {
        self.data.keys().map(|k| k.as_str()).collect()
    }

    /// Registers a scripted (Lua) job handler by name and opaque key.
    ///
    /// The key is a string understood by the scripting integration layer.
    pub fn register_lua(&mut self, name: &str, key: String) {
        self.logic
            .insert(name.to_string(), JobLogicKind::Scripted(key));
    }

    /// Loads all job types from the given directory and returns a populated registry.
    pub fn load_from_dir<P: AsRef<std::path::Path>>(
        dir: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut registry = JobTypeRegistry::new();
        let job_types = crate::systems::job::loader::load_job_types_from_dir(dir);
        for job in job_types {
            // By default, register with a no-op native handler.
            registry.register(job, JobLogicKind::Native(|_, _, _, job| job.clone()));
        }
        Ok(registry)
    }

    /// Registers a job type with the default native handler.
    pub fn register_job_type(&mut self, data: JobTypeData) {
        self.register(data, JobLogicKind::Native(|_, _, _, job| job.clone()));
    }
}
