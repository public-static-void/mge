use crate::job_bridge::{PY_JOB_HANDLER_REGISTRY, py_job_handler};
use crate::python_api::world::PyWorld;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use pythonize::depythonize;

/// Assign a new job to an entity.
pub fn assign_job(
    pyworld: &PyWorld,
    entity_id: u32,
    job_type: String,
    kwargs: Option<&Bound<'_, PyDict>>,
) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    let mut job_val = serde_json::json!({
        "id": entity_id,
        "job_type": job_type,
        "state": "pending",
        "progress": 0.0
    });
    if let Some(kwargs) = kwargs {
        let extra: serde_json::Value = depythonize(kwargs)?;
        if let Some(obj) = extra.as_object() {
            for (k, v) in obj {
                job_val[k] = v.clone();
            }
        }
    }
    world
        .set_component(entity_id, "Job", job_val)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Register a new job type with a Python callback.
pub fn register_job_type(pyworld: &PyWorld, py: Python, name: String, callback: Py<PyAny>) {
    PY_JOB_HANDLER_REGISTRY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(name.clone(), callback.clone_ref(py));

    let registry = pyworld.inner.borrow().job_handler_registry.clone();
    registry
        .lock()
        .unwrap()
        .register_handler(&name, move |world, agent_id, job_id, job_data| {
            py_job_handler(world, agent_id, job_id, job_data)
        });

    let mut world = pyworld.inner.borrow_mut();
    let _name_for_native = name.clone();
    world
        .job_types
        .register_native(&name, move |world, agent_id, job_id, job_data| {
            py_job_handler(world, agent_id, job_id, job_data)
        });
}

/// Advances the state of the job identified by `job_id`.
pub fn advance_job_state(pyworld: &PyWorld, job_id: u32) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    let job = match world.get_component(job_id, "Job") {
        Some(job) => job.clone(),
        None => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "No job with id {job_id}"
            )));
        }
    };
    let new_job =
        engine_core::systems::job::system::process::process_job(&mut world, None, job_id, job);
    world
        .set_component(job_id, "Job", new_job)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to set job: {e}")))?;
    Ok(())
}
