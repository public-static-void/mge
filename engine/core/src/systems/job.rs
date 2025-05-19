use crate::{World, ecs::system::System};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

type JobFn = dyn Fn(&JsonValue, f64) -> JsonValue + Send + Sync + 'static;

#[derive(Debug, Deserialize)]
pub struct JobTypeData {
    pub name: String,
    pub requirements: Option<Vec<String>>,
    pub duration: Option<f64>,
    pub effects: Option<Vec<serde_json::Value>>,
    // Add more fields as needed (e.g. script hooks)
}

pub enum JobLogic {
    Native(Box<JobFn>),
    Lua(mlua::RegistryKey),
}

#[derive(Default)]
pub struct JobTypeRegistry {
    logic: HashMap<String, JobLogic>,
}

impl JobTypeRegistry {
    pub fn register_native(&mut self, job_type: &str, logic: Box<JobFn>) {
        self.logic
            .insert(job_type.to_string(), JobLogic::Native(logic));
    }

    pub fn register_lua(&mut self, job_type: &str, key: mlua::RegistryKey) {
        self.logic.insert(job_type.to_string(), JobLogic::Lua(key));
    }

    pub fn register_data_job(&mut self, job: JobTypeData) {
        let logic = move |old_job: &JsonValue, progress: f64| {
            let mut job = old_job.clone();
            let status = job.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if status == "failed" || status == "complete" || status == "cancelled" {
                // Do nothing
            } else if status == "pending" {
                job["status"] = json!("in_progress");
            } else if status == "in_progress" {
                let progress = progress + 1.0;
                job["progress"] = json!(progress);
                if let Some(duration) = job.get("duration").and_then(|v| v.as_f64()).or(job
                    .get("duration")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as f64))
                {
                    if progress >= duration {
                        job["status"] = json!("complete");
                    }
                } else if progress >= 3.0 {
                    job["status"] = json!("complete");
                }
            }
            job
        };
        self.logic
            .insert(job.name.clone(), JobLogic::Native(Box::new(logic)));
    }

    pub fn get(&self, job_type: &str) -> Option<&JobLogic> {
        self.logic.get(job_type)
    }

    pub fn job_type_names(&self) -> Vec<String> {
        self.logic.keys().cloned().collect()
    }
}

#[derive(Default)]
pub struct JobSystem {
    job_types: JobTypeRegistry,
}

impl JobSystem {
    pub fn with_registry(job_types: JobTypeRegistry) -> Self {
        JobSystem { job_types }
    }

    // Recursive function to process a single job
    fn process_job(
        &self,
        _world: &mut World,
        lua: Option<&mlua::Lua>,
        _eid: u32,
        mut job: JsonValue,
    ) -> JsonValue {
        // Extract all needed fields up front
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

        // Handle cancellation up front
        if is_cancelled {
            job["status"] = json!("cancelled");
        }

        // Recursively process children, if present
        if let Some(children_val) = job.get_mut("children") {
            // Move the array out to avoid borrow checker issues
            let mut children = std::mem::take(children_val)
                .as_array_mut()
                .map(std::mem::take)
                .unwrap_or_default();

            let mut all_children_complete = true;
            for child in &mut children {
                // Recursively process each child
                let processed = self.process_job(_world, lua, _eid, child.take());
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
            // Put the array back into the parent job
            *children_val = JsonValue::Array(children);

            // If all children are complete and not cancelled, mark parent complete
            if !is_cancelled && all_children_complete {
                job["status"] = json!("complete");
            }
        }

        // Only apply job logic if not cancelled or terminal
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
                    // fallback logic
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
            // Overwrite all fields except children (which we've already processed)
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

pub fn load_job_types_from_dir<P: AsRef<Path>>(dir: P) -> Vec<JobTypeData> {
    let mut jobs = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return jobs, // Gracefully handle missing directory
    };
    for entry in entries {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let data = fs::read_to_string(&path).expect("Failed to read job file");
            let job: JobTypeData = serde_json::from_str(&data).expect("Failed to parse job file");
            jobs.push(job);
        }
        if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            let data = fs::read_to_string(&path).expect("Failed to read job file");
            let job: JobTypeData =
                serde_yaml::from_str(&data).expect("Failed to parse YAML job file");
            jobs.push(job);
        }
        if path.extension().is_some_and(|e| e == "toml") {
            let data = fs::read_to_string(&path).expect("Failed to read job file");
            let job: JobTypeData = toml::from_str(&data).expect("Failed to parse TOML job file");
            jobs.push(job);
        }
    }
    jobs
}
