use crate::ecs::world::World;
use crate::systems::job::loader::load_job_types_from_dir;
use crate::systems::job::registry::{JobLogic, JobTypeData};
use std::path::Path;

pub fn register_builtin_job_handlers(world: &mut World, jobs_dir: &Path) {
    let job_types: Vec<JobTypeData> = load_job_types_from_dir(jobs_dir);

    for job_type in job_types {
        let job_type_name = job_type.name.clone();
        match &job_type.logic {
            JobLogic::Native(logic) => {
                // Register native Rust logic
                world.job_handler_registry.register_handler(
                    &job_type_name,
                    move |world, agent_id, job_id| {
                        let mut job = world.get_component(job_id, "Job").cloned().unwrap();
                        let progress =
                            job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 1.0;
                        job["progress"] = serde_json::json!(progress);
                        if progress >= 3.0 {
                            job["status"] = serde_json::json!("complete");
                        } else {
                            job["status"] = serde_json::json!("in_progress");
                        }
                        world.set_component(job_id, "Job", job).unwrap();
                    },
                );
            }
            JobLogic::Lua(lua_key) => {
                // Register Lua handler (if you support scripting)
                // Example: world.job_handler_registry.register_lua_handler(&job_type_name, lua_key.clone());
            }
            JobLogic::None => {
                // No-op or error handler
            }
        }
    }
}
