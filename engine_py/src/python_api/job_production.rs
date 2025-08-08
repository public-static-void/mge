use crate::python_api::world::PyWorld;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Get the production job component for an entity.
pub fn get_production_job(
    pyworld: &PyWorld,
    py: Python,
    entity_id: u32,
) -> PyResult<Option<PyObject>> {
    let world = pyworld.inner.borrow();
    if let Some(job) = world.get_component(entity_id, "ProductionJob") {
        Ok(Some(serde_pyobject::to_pyobject(py, job)?.into()))
    } else {
        Ok(None)
    }
}

/// Get the progress value of a production job by entity ID.
pub fn get_production_job_progress(pyworld: &PyWorld, entity_id: u32) -> PyResult<i64> {
    let world = pyworld.inner.borrow();
    if let Some(job) = world.get_component(entity_id, "ProductionJob") {
        Ok(job.get("progress").and_then(|v| v.as_i64()).unwrap_or(0))
    } else {
        Ok(0)
    }
}

/// Set the progress value of a production job by entity ID.
pub fn set_production_job_progress(pyworld: &PyWorld, entity_id: u32, value: i64) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    if let Some(mut job) = world.get_component(entity_id, "ProductionJob").cloned() {
        job["progress"] = serde_json::json!(value);
        world
            .set_component(entity_id, "ProductionJob", job)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
    }
    Ok(())
}

/// Get the state string of a production job by entity ID.
pub fn get_production_job_state(pyworld: &PyWorld, entity_id: u32) -> PyResult<String> {
    let world = pyworld.inner.borrow();
    if let Some(job) = world.get_component(entity_id, "ProductionJob") {
        Ok(job
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string())
    } else {
        Ok("pending".to_string())
    }
}

/// Set the state string of a production job by entity ID.
pub fn set_production_job_state(pyworld: &PyWorld, entity_id: u32, value: String) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    if let Some(mut job) = world.get_component(entity_id, "ProductionJob").cloned() {
        job["state"] = serde_json::json!(value);
        world
            .set_component(entity_id, "ProductionJob", job)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
    }
    Ok(())
}
