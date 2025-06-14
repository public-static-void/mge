use crate::ecs::world::World;

impl World {
    /// Register a custom job handler for a job type.
    pub fn register_job_handler<F>(&mut self, job_type: &str, handler: F)
    where
        F: Fn(&World, u32, u32, &serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.job_handler_registry
            .lock()
            .unwrap()
            .register_handler(job_type, handler);
    }
}
