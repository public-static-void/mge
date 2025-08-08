use crate::python_api::world::PyWorld;
use pyo3::PyObject;
use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::prelude::*;

/// Get the dependencies field for a job by ID.
pub fn get_job_dependencies(pyworld: &PyWorld, py: Python, job_id: u32) -> PyResult<PyObject> {
    let world = pyworld.inner.borrow();
    let job = world
        .get_component(job_id, "Job")
        .ok_or_else(|| PyKeyError::new_err(format!("No job with id {job_id}")))?;
    let deps = job
        .get("dependencies")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(serde_pyobject::to_pyobject(py, &deps)?.into())
}

/// Set the dependencies field for a job by ID.
///
/// **Note:** `dependencies` here is the Python-bound object with GIL lifetime.
pub fn set_job_dependencies(
    pyworld: &PyWorld,
    job_id: u32,
    dependencies: Bound<'_, PyAny>,
) -> PyResult<()> {
    // Convert Python object with bound lifetime to serde_json::Value
    let deps_json: serde_json::Value = serde_pyobject::from_pyobject(dependencies)?;

    let mut world = pyworld.inner.borrow_mut();
    let mut job = world
        .get_component(job_id, "Job")
        .cloned()
        .ok_or_else(|| PyKeyError::new_err(format!("No job with id {job_id}")))?;
    job["dependencies"] = deps_json;
    world
        .set_component(job_id, "Job", job)
        .map_err(|e| PyValueError::new_err(format!("Failed to set job: {e}")))
}
