use crate::ecs::world::World;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for job handler closures that can mutate the world.
///
/// Handlers are invoked every tick a job is running (see [`process_job_progress`]).
/// They may mutate job state, world state, or trigger effects.
///
/// # Arguments
///
/// - `world`: The ECS world, mutably borrowed (`&mut World`)
/// - `agent_id`: The assigned agent entity ID, or `0` if unassigned (`u32`)
/// - `job_id`: The job entity ID (`u32`)
/// - `data`: The current job data (`&serde_json::Value`)
///
/// # Returns
///
/// Returns the new value of the job component as a [`serde_json::Value`] (may be unchanged).
pub type JobHandler =
    Arc<dyn Fn(&mut World, u32, u32, &serde_json::Value) -> serde_json::Value + Send + Sync>;

fn normalize_key(key: &str) -> String {
    key.trim().to_lowercase().replace(' ', "_")
}

/// Registry for per-job-type logic handlers.
#[derive(Default)]
pub struct JobHandlerRegistry {
    handlers: HashMap<String, JobHandler>,
}

impl JobHandlerRegistry {
    /// Creates a new job handler registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Registers a handler for the given job type.
    ///
    /// The handler will be invoked during job progress for jobs of this type.
    pub fn register_handler<F>(&mut self, job_type: &str, handler: F)
    where
        F: Fn(&mut World, u32, u32, &serde_json::Value) -> serde_json::Value
            + Send
            + Sync
            + 'static,
    {
        let key = normalize_key(job_type);
        self.handlers.insert(key, Arc::new(handler));
    }

    /// Returns a reference to the handler for the job type, if present.
    pub fn get(&self, job_type: &str) -> Option<&JobHandler> {
        let key = normalize_key(job_type);
        self.handlers.get(&key)
    }

    /// Lists all registered job type keys.
    pub fn keys(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}
