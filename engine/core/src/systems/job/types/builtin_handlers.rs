use crate::ecs::world::World;
use crate::systems::job::types::job_type::{JobLogicKind, JobTypeData, JobTypeRegistry};
use crate::systems::job::types::loader::load_job_types_from_dir;
use std::path::Path;

/// Registers all native job handlers for job types found in the given directory.
/// Only job types registered as Native in the JobTypeRegistry will have handlers registered.
pub fn register_builtin_job_handlers(
    world: &mut World,
    job_type_registry: &JobTypeRegistry,
    jobs_dir: &Path,
) {
    // Load job types from directory
    let job_types: Vec<JobTypeData> = load_job_types_from_dir(jobs_dir);

    // For each job type, check if there is native logic registered, and register handler if so
    for job_type in job_types {
        let job_type_name = job_type.name.clone();
        // Look up logic in the provided registry
        let logic_opt = job_type_registry.get_logic(&job_type_name);

        if let Some(JobLogicKind::Native(_logic_fn)) = logic_opt {
            world.job_handler_registry.lock().unwrap().register_handler(
                &job_type_name,
                move |world, _agent_id, job_id, _data| {
                    let mut job = world.get_component(job_id, "Job").cloned().unwrap();
                    let progress =
                        job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
                    job["progress"] = serde_json::json!(progress);
                    if progress >= 3.0 {
                        job["state"] = serde_json::json!("complete");
                    } else {
                        job["state"] = serde_json::json!("in_progress");
                    }
                    // Do NOT mutate world here, just return the job value
                    job
                },
            );
        }
        // Handle Lua or Data jobs if needed
    }
}
