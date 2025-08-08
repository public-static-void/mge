use crate::python_api::world::PyWorld;
use pyo3::PyObject;
use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;

/// Get the children array (list of job objects) for a job by ID.
pub fn get_job_children(pyworld: &PyWorld, py: Python, job_id: u32) -> PyResult<PyObject> {
    let world = pyworld.inner.borrow();
    let job = world
        .get_component(job_id, "Job")
        .ok_or_else(|| PyKeyError::new_err(format!("No job with id {job_id}")))?;
    let children = job
        .get("children")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    Ok(serde_pyobject::to_pyobject(py, &children)?.into())
}

/// Set the children array (list of job objects) for a job by ID.
///
/// **Note:** `children` here is the Python-bound object with GIL lifetime.
pub fn set_job_children(
    pyworld: &PyWorld,
    job_id: u32,
    children: Bound<'_, PyAny>,
) -> PyResult<()> {
    // Convert Python object with bound lifetime to serde_json::Value
    let children_json: serde_json::Value = serde_pyobject::from_pyobject(children)?;

    let mut world = pyworld.inner.borrow_mut();
    let mut job = world
        .get_component(job_id, "Job")
        .cloned()
        .ok_or_else(|| PyKeyError::new_err(format!("No job with id {job_id}")))?;
    job["children"] = children_json;
    world
        .set_component(job_id, "Job", job)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to set job: {e}")))
}
