use crate::ecs::world::World;
use std::collections::HashMap;
use std::sync::Arc;

pub type JobHandler =
    Arc<dyn Fn(&World, u32, u32, &serde_json::Value) -> serde_json::Value + Send + Sync>;

fn normalize_key(key: &str) -> String {
    key.trim().to_lowercase().replace(' ', "_")
}

#[derive(Default)]
pub struct JobHandlerRegistry {
    handlers: HashMap<String, JobHandler>,
}

impl JobHandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register_handler<F>(&mut self, job_type: &str, handler: F)
    where
        F: Fn(&World, u32, u32, &serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        let key = normalize_key(job_type);
        self.handlers.insert(key, Arc::new(handler));
    }

    pub fn get(&self, job_type: &str) -> Option<&JobHandler> {
        let key = normalize_key(job_type);
        self.handlers.get(&key)
    }

    pub fn keys(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}
