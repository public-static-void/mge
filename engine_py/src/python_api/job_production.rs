use crate::PyObject;
use crate::python_api::world::PyWorld;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde_json::json;

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

/// Enqueue a production job on an entity.
/// Returns true if enqueued, false if entity already has a ProductionJob.
pub fn enqueue_production_job(
    pyworld: &PyWorld,
    entity_id: u32,
    recipe_name: String,
    priority: Option<i64>,
    batch_size: Option<i64>,
) -> PyResult<bool> {
    let mut world = pyworld.inner.borrow_mut();
    // Check if entity already has a ProductionJob
    if world.get_component(entity_id, "ProductionJob").is_some() {
        return Ok(false);
    }
    let priority = priority.unwrap_or(0);
    let batch_size = batch_size.filter(|&v| v >= 1).unwrap_or(1);
    let job = json!({
        "recipe": recipe_name,
        "progress": 0,
        "state": "pending",
        "priority": priority,
        "batch_size": batch_size,
    });
    world
        .set_component(entity_id, "ProductionJob", job)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(true)
}

/// Get the production queue (single job) for an entity.
/// Returns a dict with {recipe, progress, state, priority, batch_size} or None.
pub fn get_production_queue(
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

/// Get completed production jobs for an entity (polling).
/// Returns a list of completion event payloads. Clears consumed events.
pub fn get_completed_production_jobs(
    pyworld: &PyWorld,
    py: Python,
    entity_id: u32,
) -> PyResult<Vec<PyObject>> {
    let mut world = pyworld.inner.borrow_mut();
    let all_events = world.take_events("production_completed");
    let filtered: Vec<serde_json::Value> = all_events
        .into_iter()
        .filter(|ev| ev.get("entity").and_then(|v| v.as_u64()) == Some(entity_id as u64))
        .collect();
    let results: Vec<PyObject> = filtered
        .into_iter()
        .map(|v| serde_pyobject::to_pyobject(py, &v).map(|o| o.into()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(results)
}
