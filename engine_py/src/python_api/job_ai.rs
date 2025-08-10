use crate::python_api::world::PyWorld;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Assign jobs to an AI agent using the internal job AI logic.
pub fn ai_assign_jobs(pyworld: &PyWorld, agent_id: u32, _args: Vec<PyObject>) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();

    let job_board_ptr: *mut _ = &mut world.job_board;
    use engine_core::systems::job::ai::logic::assign_jobs;

    unsafe {
        assign_jobs(&mut world, &mut *job_board_ptr, agent_id as u64, &[]);
    }

    Ok(())
}

/// Query all jobs assigned to a given AI agent.
pub fn ai_query_jobs(pyworld: &PyWorld, py: Python<'_>, agent_id: u32) -> PyResult<PyObject> {
    let world = pyworld.inner.borrow();
    let mut jobs_py: Vec<Py<PyAny>> = Vec::new();

    if let Some(job_map) = world.components.get("Job") {
        for (&job_id, job_comp) in job_map.iter() {
            if let Some(assigned_to) = job_comp.get("assigned_to").and_then(|v| v.as_u64())
                && assigned_to == agent_id as u64
            {
                let dict = PyDict::new(py);
                dict.set_item("id", job_id)?;
                dict.set_item(
                    "state",
                    job_comp.get("state").and_then(|v| v.as_str()).unwrap_or(""),
                )?;
                dict.set_item(
                    "job_type",
                    job_comp
                        .get("job_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                )?;
                dict.set_item("assigned_to", assigned_to)?;
                jobs_py.push(dict.into());
            }
        }
    }

    Ok(PyList::new(py, jobs_py)?.into())
}

/// Modify a job assignment for an AI-controlled agent.
pub fn ai_modify_job_assignment(
    pyworld: &PyWorld,
    job_id: u32,
    kwargs: Option<&Bound<'_, PyDict>>,
) -> PyResult<bool> {
    let mut world = pyworld.inner.borrow_mut();

    // Get the job component or error if missing
    let mut job = world
        .get_component(job_id, "Job")
        .ok_or_else(|| PyValueError::new_err(format!("No job with id {job_id}")))?
        .clone();

    if let Some(kwargs_dict) = kwargs {
        for (key, value) in kwargs_dict.iter() {
            let k: String = key.extract()?;
            let v: serde_json::Value = pythonize::depythonize(&value)?;
            job[k] = v;
        }
    }

    // Persist updated job component
    world
        .set_component(job_id, "Job", job)
        .map_err(|e| PyValueError::new_err(format!("Failed to set job: {e}")))?;

    Ok(true)
}
