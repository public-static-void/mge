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
        let job_types = crate::systems::job::types::loader::load_job_types_from_dir(dir);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Dummy handler for tests that returns the input unchanged.
    fn dummy_handler(
        _world: &mut crate::ecs::world::World,
        _entity: u32,
        _job_id: u32,
        job_data: &serde_json::Value,
    ) -> serde_json::Value {
        job_data.clone()
    }

    fn make_data(name: &str) -> JobTypeData {
        JobTypeData {
            name: name.to_string(),
            requirements: vec!["skill_a".into()],
            duration: Some(5.0),
            effects: vec![json!({"action": "test"})],
        }
    }

    // --- Registration round-trips ---

    #[test]
    fn test_register_get_data_native() {
        let mut reg = JobTypeRegistry::new();
        let data = make_data("test_job");
        reg.register(data.clone(), JobLogicKind::Native(dummy_handler));

        let retrieved = reg.get_data("test_job");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test_job");

        let logic = reg.get_logic("test_job");
        assert!(logic.is_some());
        assert!(matches!(logic.unwrap(), JobLogicKind::Native(_)));
    }

    #[test]
    fn test_register_get_logic_scripted() {
        let mut reg = JobTypeRegistry::new();
        let data = make_data("scripted_job");
        reg.register(data, JobLogicKind::Scripted("my_key".into()));

        let retrieved = reg.get_data("scripted_job");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "scripted_job");

        let logic = reg.get_logic("scripted_job");
        assert!(logic.is_some());
        match logic.unwrap() {
            JobLogicKind::Scripted(key) => assert_eq!(key, "my_key"),
            _ => panic!("Expected Scripted"),
        }
    }

    // --- Convenience registration ---

    #[test]
    fn test_register_native_creates_defaults() {
        let mut reg = JobTypeRegistry::new();
        reg.register_native("default_job", dummy_handler);

        let data = reg.get_data("default_job");
        assert!(data.is_some());
        let d = data.unwrap();
        assert_eq!(d.name, "default_job");
        assert!(d.requirements.is_empty());
        assert!(d.duration.is_none());
        assert!(d.effects.is_empty());

        let logic = reg.get_logic("default_job");
        assert!(matches!(logic.unwrap(), JobLogicKind::Native(_)));
    }

    #[test]
    fn test_register_lua() {
        let mut reg = JobTypeRegistry::new();
        reg.register_lua("lua_job", "lua_key".into());

        // register_lua only inserts into the logic map
        assert!(reg.get_data("lua_job").is_none());

        let logic = reg.get_logic("lua_job");
        assert!(logic.is_some());
        match logic.unwrap() {
            JobLogicKind::Scripted(key) => assert_eq!(key, "lua_key"),
            _ => panic!("Expected Scripted"),
        }
    }

    #[test]
    fn test_register_job_type() {
        let mut reg = JobTypeRegistry::new();
        let data = make_data("auto_job");
        reg.register_job_type(data);

        let retrieved = reg.get_data("auto_job");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "auto_job");

        let logic = reg.get_logic("auto_job");
        assert!(matches!(logic.unwrap(), JobLogicKind::Native(_)));
    }

    // --- Name enumeration ---

    #[test]
    fn test_job_type_names() {
        let mut reg = JobTypeRegistry::new();
        reg.register_native("alpha", dummy_handler);
        reg.register_native("beta", dummy_handler);

        let mut names = reg.job_type_names();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_job_type_names_uses_data_map() {
        let mut reg = JobTypeRegistry::new();
        // register_lua only adds to logic, not data
        reg.register_lua("only_logic", "key".into());
        assert!(reg.job_type_names().is_empty());

        // registering via register adds to both
        reg.register_native("both", dummy_handler);
        assert_eq!(reg.job_type_names(), vec!["both"]);
    }

    #[test]
    fn test_empty_registry_no_names() {
        let reg = JobTypeRegistry::new();
        assert!(reg.job_type_names().is_empty());
    }

    // --- Error / edge paths ---

    #[test]
    fn test_get_data_unregistered() {
        let reg = JobTypeRegistry::new();
        assert!(reg.get_data("nonexistent").is_none());
    }

    #[test]
    fn test_get_logic_unregistered() {
        let reg = JobTypeRegistry::new();
        assert!(reg.get_logic("nonexistent").is_none());
    }

    #[test]
    fn test_name_in_logic_only() {
        let mut reg = JobTypeRegistry::new();
        reg.register_lua("logic_only", "key".into());
        assert!(reg.get_data("logic_only").is_none());
        assert!(reg.get_logic("logic_only").is_some());
    }

    #[test]
    fn test_register_overwrite() {
        let mut reg = JobTypeRegistry::new();
        let first = make_data("overwrite");
        let mut second = make_data("overwrite");
        second.duration = Some(99.0);

        reg.register(first, JobLogicKind::Native(dummy_handler));
        reg.register(second, JobLogicKind::Scripted("new".into()));

        let data = reg.get_data("overwrite").unwrap();
        assert_eq!(data.duration, Some(99.0));

        let logic = reg.get_logic("overwrite").unwrap();
        assert!(matches!(logic, JobLogicKind::Scripted(_)));
    }

    // --- Deserialization round-trips ---

    #[test]
    fn test_job_type_data_roundtrip_json() {
        let data = make_data("roundtrip_json");
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: JobTypeData = serde_json::from_str(&json).unwrap();
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_job_type_data_roundtrip_toml() {
        let data = make_data("roundtrip_toml");
        let toml_str = toml::to_string(&data).unwrap();
        let deserialized: JobTypeData = toml::from_str(&toml_str).unwrap();
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_job_effect_deserialize() {
        let json = r#"{"action": "move", "from": "A", "to": "B"}"#;
        let effect: JobEffect = serde_json::from_str(json).unwrap();
        assert_eq!(effect.action, "move");
        assert_eq!(effect.from, Some("A".to_string()));
        assert_eq!(effect.to, Some("B".to_string()));
    }

    // --- Default-value edge cases ---

    #[test]
    fn test_job_type_data_defaults() {
        // minimal JSON — missing optional fields
        let json = r#"{"name": "minimal"}"#;
        let data: JobTypeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.name, "minimal");
        assert!(data.requirements.is_empty());
        assert!(data.duration.is_none());
        assert!(data.effects.is_empty());
    }
}
