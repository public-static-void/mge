use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pythonize::{depythonize, pythonize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Job handler registry
pub static PY_JOB_HANDLER_REGISTRY: Lazy<Mutex<HashMap<String, Py<PyAny>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Register a job handler
pub fn py_job_handler(
    _world: &engine_core::ecs::world::World,
    _agent_id: u32,
    _job_id: u32,
    job_data: &serde_json::Value,
) -> serde_json::Value {
    Python::with_gil(|py| {
        let job_type = job_data
            .get("job_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let registry = PY_JOB_HANDLER_REGISTRY
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        if let Some(cb) = registry.get(job_type) {
            let job_obj = pythonize(py, job_data).unwrap();

            match cb.call1(py, (job_obj,)) {
                Ok(res) => {
                    let result_bound = res.bind(py);
                    depythonize(result_bound).unwrap()
                }
                Err(e) => {
                    e.print(py);
                    job_data.clone()
                }
            }
        } else {
            job_data.clone()
        }
    })
}
