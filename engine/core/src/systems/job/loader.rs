//! Job type loader for loading job definitions from disk.

use std::fs;
use std::path::Path;

use super::registry::JobTypeData;

/// Loads all job types from the given directory, supporting JSON, YAML, and TOML formats.
pub fn load_job_types_from_dir<P: AsRef<Path>>(dir: P) -> Vec<JobTypeData> {
    let mut jobs = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return jobs,
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
