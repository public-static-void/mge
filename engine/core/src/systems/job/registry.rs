use serde::Deserialize;
use std::collections::HashMap;

pub type JobFn = dyn Fn(&serde_json::Value, f64) -> serde_json::Value + Send + Sync + 'static;

#[derive(Debug, Deserialize)]
pub struct JobTypeData {
    pub name: String,
    pub requirements: Option<Vec<String>>,
    pub duration: Option<f64>,
    pub effects: Option<Vec<serde_json::Value>>,
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
        let logic = move |old_job: &serde_json::Value, progress: f64| {
            let mut job = old_job.clone();
            let status = job.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if status == "failed" || status == "complete" || status == "cancelled" {
            } else if status == "pending" {
                job["status"] = serde_json::json!("in_progress");
            } else if status == "in_progress" {
                let progress = progress + 1.0;
                job["progress"] = serde_json::json!(progress);
                if let Some(duration) = job.get("duration").and_then(|v| v.as_f64()).or(job
                    .get("duration")
                    .and_then(|v| v.as_i64())
                    .map(|i| i as f64))
                {
                    if progress >= duration {
                        job["status"] = serde_json::json!("complete");
                    }
                } else if progress >= 3.0 {
                    job["status"] = serde_json::json!("complete");
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
