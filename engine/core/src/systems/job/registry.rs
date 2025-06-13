use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// The function signature for native job logic.
pub type JobFn = dyn Fn(&serde_json::Value, f64) -> serde_json::Value + Send + Sync + 'static;

/// Data structure for a job type loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobTypeData {
    pub name: String,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub effects: Vec<serde_json::Value>,
}

/// Enum for the logic associated with a job type.
pub enum JobLogic {
    Native(Box<JobFn>),
    Lua(mlua::RegistryKey),
    Data,
}

#[derive(Default)]
pub struct JobTypeRegistry {
    // Keyed by normalized job type name.
    types: HashMap<String, JobTypeData>,
    logic: HashMap<String, JobLogic>,
    names: HashSet<String>, // Set of all registered job type names with original casing
}

impl JobTypeRegistry {
    /// Loads all job types from a directory of JSON files.
    pub fn load_from_dir(dir: &Path) -> Result<Self, String> {
        let mut types = HashMap::new();
        let mut names = HashSet::new();
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let job_type: JobTypeData =
                    serde_json::from_str(&data).map_err(|e| e.to_string())?;
                let key = Self::normalize_key(&job_type.name);
                names.insert(job_type.name.clone());
                types.insert(key, job_type);
            }
        }
        Ok(JobTypeRegistry {
            types,
            logic: HashMap::new(),
            names,
        })
    }

    /// Registers a native Rust function as job logic.
    pub fn register_native(&mut self, job_type: &str, logic: Box<JobFn>) {
        let key = Self::normalize_key(job_type);
        self.logic.insert(key, JobLogic::Native(logic));
        self.names.insert(job_type.to_string());
    }

    /// Registers a Lua function as job logic.
    pub fn register_lua(&mut self, job_type: &str, key: mlua::RegistryKey) {
        let norm = Self::normalize_key(job_type);
        self.logic.insert(norm, JobLogic::Lua(key));
        self.names.insert(job_type.to_string());
    }

    /// Registers a data-driven job (uses default progress/duration logic).
    pub fn register_data_job(&mut self, job: JobTypeData) {
        let key = Self::normalize_key(&job.name);
        self.names.insert(job.name.clone());
        self.types.insert(key.clone(), job);
        self.logic.insert(key, JobLogic::Data);
    }

    /// Register a job type with the given name and effects.
    pub fn register_job_type(&mut self, name: &str, effects: Vec<Value>) {
        let key = Self::normalize_key(name);
        self.types.insert(
            key,
            JobTypeData {
                name: name.to_string(),
                effects,
                requirements: Vec::new(),
                duration: None,
            },
        );
    }

    /// Gets the logic for a job type, if registered.
    pub fn get_logic(&self, job_type: &str) -> Option<&JobLogic> {
        self.logic.get(&Self::normalize_key(job_type))
    }

    /// Gets the data for a job type, if loaded.
    pub fn get_data(&self, job_type: &str) -> Option<&JobTypeData> {
        self.types.get(&Self::normalize_key(job_type))
    }

    /// Returns a Vec of all job type names (as Strings, original casing).
    pub fn job_type_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.names.iter().cloned().collect();
        names.sort();
        names
    }

    fn normalize_key(key: &str) -> String {
        key.trim().to_lowercase().replace(' ', "_")
    }
}
