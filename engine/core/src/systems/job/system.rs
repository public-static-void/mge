use crate::World;
use crate::ecs::system::System;
use serde_json::{Value as JsonValue, json};

use super::registry::{JobLogic, JobTypeRegistry};

#[derive(Default)]
pub struct JobSystem {
    job_types: JobTypeRegistry,
}

impl JobSystem {
    pub fn with_registry(job_types: JobTypeRegistry) -> Self {
        JobSystem { job_types }
    }

    fn dependencies_complete(&self, world: &World, job: &JsonValue) -> bool {
        if let Some(deps) = job.get("dependencies").and_then(|v| v.as_array()) {
            for dep in deps {
                if let Some(dep_id) = dep.as_str() {
                    if let Ok(dep_eid) = dep_id.parse::<u32>() {
                        if let Some(dep_job) = world.get_component(dep_eid, "Job") {
                            let status =
                                dep_job.get("status").and_then(|v| v.as_str()).unwrap_or("");
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

    fn process_job(
        &self,
        world: &mut World,
        lua: Option<&mlua::Lua>,
        _eid: u32,
        mut job: JsonValue,
    ) -> JsonValue {
        let job_type = job
            .get("job_type")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let is_cancelled = job
            .get("cancelled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let status = job.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status == "pending" && !self.dependencies_complete(world, &job) {
            return job;
        }

        if is_cancelled {
            job["status"] = json!("cancelled");
        }

        if let Some(children_val) = job.get_mut("children") {
            let mut children = std::mem::take(children_val)
                .as_array_mut()
                .map(std::mem::take)
                .unwrap_or_default();
            let mut all_children_complete = true;
            for child in &mut children {
                let processed = self.process_job(world, lua, _eid, child.take());
                if is_cancelled {
                    *child = processed;
                    child["status"] = json!("cancelled");
                } else {
                    *child = processed;
                }
                if child.get("status").and_then(|v| v.as_str()) != Some("complete") {
                    all_children_complete = false;
                }
            }
            *children_val = JsonValue::Array(children);

            if !is_cancelled && all_children_complete {
                job["status"] = json!("complete");
            }
        }

        let status = job.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if !matches!(status, "failed" | "complete" | "cancelled") {
            let new_job = match self.job_types.get(&job_type) {
                Some(JobLogic::Native(logic)) => logic(&job, progress),
                Some(JobLogic::Lua(key)) => {
                    use mlua::LuaSerdeExt;
                    let lua = lua.expect("Lua context required for Lua job logic");
                    let job_table = lua.to_value(&job).unwrap();
                    let func: mlua::Function = lua.registry_value(key).unwrap();
                    let result = func.call((job_table, progress)).unwrap();
                    lua.from_value(result).unwrap()
                }
                None => {
                    let mut job = job.clone();
                    if job
                        .get("should_fail")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        job["status"] = json!("failed");
                    } else if status == "pending" {
                        job["status"] = json!("in_progress");
                    } else if status == "in_progress" {
                        let progress = progress + 1.0;
                        job["progress"] = json!(progress);
                        if progress >= 3.0 {
                            job["status"] = json!("complete");
                        }
                    }
                    job
                }
            };
            if job.get("children").is_some() {
                let children = job.get("children").cloned();
                job = new_job;
                if let Some(children) = children {
                    job["children"] = children;
                }
            } else {
                job = new_job;
            }
        }

        job
    }
}

impl System for JobSystem {
    fn name(&self) -> &'static str {
        "JobSystem"
    }

    fn run(&mut self, world: &mut World, lua: Option<&mlua::Lua>) {
        let entities = world.get_entities_with_component("Job");
        for eid in entities {
            let old_job = world.get_component(eid, "Job").unwrap().clone();
            let old_status = old_job
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let job = self.process_job(world, lua, eid, old_job);
            let new_status = job.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if old_status != new_status {
                if new_status == "complete" {
                    let event = json!({
                        "entity": eid,
                        "job_type": job.get("job_type").cloned().unwrap_or(json!(null))
                    });
                    let _ = world.send_event("job_completed", event);
                } else if new_status == "failed" {
                    let event = json!({
                        "entity": eid,
                        "job_type": job.get("job_type").cloned().unwrap_or(json!(null))
                    });
                    let _ = world.send_event("job_failed", event);
                } else if new_status == "cancelled" {
                    let event = json!({
                        "entity": eid,
                        "job_type": job.get("job_type").cloned().unwrap_or(json!(null))
                    });
                    let _ = world.send_event("job_cancelled", event);
                }
            }
            world.set_component(eid, "Job", job).unwrap();
        }
    }
}
