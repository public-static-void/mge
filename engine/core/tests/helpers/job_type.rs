use engine_core::systems::job::types::job_type::JobTypeData;
use std::fs;
use std::path::Path;

/// Loads a job type JSON file from disk and parses it as JobTypeData.
/// Panics if the file cannot be read or parsed.
pub fn load_job_type_from_assets(name: &str) -> JobTypeData {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../engine/assets/jobs")
        .join(format!("{name}.json"));
    let data =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read job file: {path:?}"));
    serde_json::from_str(&data)
        .unwrap_or_else(|_| panic!("Failed to parse JobTypeData from {path:?}"))
}
