use crate::ecs::world::World;

/// Extension methods for registering custom job handlers.
impl World {
    /// Register a custom job handler for a job type.
    ///
    /// The handler can mutate the world on each job tick. Only one handler per job type
    /// is allowed; registering another will replace the previous handler.
    ///
    /// # Arguments
    /// * `job_type` - The name of the job type to handle.
    /// * `handler`  - A closure that receives a mutable world, the assigned agent, job entity id, and the job component.
    pub fn register_job_handler<F>(&mut self, job_type: &str, handler: F)
    where
        F: Fn(&mut World, u32, u32, &serde_json::Value) -> serde_json::Value
            + Send
            + Sync
            + 'static,
    {
        self.job_handler_registry
            .lock()
            .unwrap()
            .register_handler(job_type, handler);
    }
}
