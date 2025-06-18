use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// Returns true if all dependencies of the job are complete.
pub fn dependencies_complete(world: &World, job: &JsonValue) -> bool {
    if let Some(deps) = job.get("dependencies").and_then(|v| v.as_array()) {
        for dep in deps {
            if let Some(dep_id) = dep.as_str() {
                if let Ok(dep_eid) = dep_id.parse::<u32>() {
                    if let Some(dep_job) = world.get_component(dep_eid, "Job") {
                        let status = dep_job.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        if status != "complete" {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
    }
    true
}

/// Returns Some("failed") or Some("cancelled") if any dependency has failed or been cancelled, otherwise None.
pub fn dependency_failure_status(world: &World, job: &JsonValue) -> Option<&'static str> {
    if let Some(deps) = job.get("dependencies").and_then(|v| v.as_array()) {
        for dep in deps {
            if let Some(dep_id) = dep.as_str() {
                if let Ok(dep_eid) = dep_id.parse::<u32>() {
                    if let Some(dep_job) = world.get_component(dep_eid, "Job") {
                        let status = dep_job.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        if status == "failed" {
                            return Some("failed");
                        }
                        if status == "cancelled" {
                            return Some("cancelled");
                        }
                    }
                }
            }
        }
    }
    None
}
