use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::{Value as JsonValue, json};

#[derive(Default)]
pub struct JobSystem;

impl JobSystem {
    pub fn new() -> Self {
        JobSystem
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
        _lua: Option<&mlua::Lua>,
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
                let processed = self.process_job(world, _lua, _eid, child.take());
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
            let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

            if let Some(handler) = world.job_handler_registry.get(&job_type) {
                let handler = handler.clone();
                handler(world, assigned_to, job_id, &job)
            } else {
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
        } else {
            job
        }
    }

    fn cleanup_agent_on_job_status(&self, world: &mut World, job: &JsonValue) {
        if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64()) {
            let agent_id = agent_id as u32;
            if let Some(agent) = world
                .components
                .get_mut("Agent")
                .and_then(|m| m.get_mut(&agent_id))
            {
                if agent.get("current_job").and_then(|v| v.as_u64())
                    == job.get("id").and_then(|v| v.as_u64())
                {
                    agent.as_object_mut().unwrap().remove("current_job");
                    agent["state"] = JsonValue::from("idle");
                }
            }
        }
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
                if matches!(new_status, "complete" | "failed" | "cancelled") {
                    self.cleanup_agent_on_job_status(world, &job);
                }
                if new_status == "complete" {
                    let event = json!({
                        "entity": eid,
                        "job_type": job.get("job_type").cloned().unwrap_or(json!(null))
                    });
                    let _ = world.send_event("job_completed", event);

                    let mut effects: Vec<serde_json::Value> = Vec::new();
                    let job_type_name_opt = job.get("job_type").and_then(|v| v.as_str());
                    if let Some(job_type_name) = job_type_name_opt {
                        if let Some(job_type) = world.job_types.get_data(job_type_name) {
                            for e in &job_type.effects {
                                effects.push(serde_json::to_value(e).unwrap());
                            }
                        }
                    }
                    if !effects.is_empty() {
                        let mut registry = world
                            .effect_processor_registry
                            .take()
                            .expect("EffectProcessorRegistry missing");
                        registry.process_effects(world, eid, &effects);
                        world.effect_processor_registry = Some(registry);
                    }
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
