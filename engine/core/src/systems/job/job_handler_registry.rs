use crate::ecs::world::World;
use std::collections::HashMap;

pub type JobHandler =
    Box<dyn Fn(&mut World, u32, u32, &serde_json::Value) -> serde_json::Value + Send + Sync>;

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
        F: Fn(&mut World, u32, u32, &serde_json::Value) -> serde_json::Value
            + Send
            + Sync
            + 'static,
    {
        self.handlers
            .insert(job_type.to_string(), Box::new(handler));
    }

    pub fn get(&self, job_type: &str) -> Option<&JobHandler> {
        self.handlers.get(job_type)
    }
}
