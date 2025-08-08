use crate::python_api::world::PyWorld;
use pyo3::PyObject;
use pyo3::Python;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;

/// Returns a list of all registered job type names.
pub fn get_job_types(pyworld: &PyWorld) -> PyResult<Vec<String>> {
    let world = pyworld.inner.borrow();
    Ok(world
        .job_types
        .job_type_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect())
}

/// Get the metadata for a job type by name.
/// Returns the job type data as a Python dict, or None if not found.
pub fn get_job_type_metadata(
    pyworld: &PyWorld,
    py: Python,
    name: String,
) -> PyResult<Option<PyObject>> {
    let world = pyworld.inner.borrow();
    if let Some(data) = world.job_types.get_data(&name) {
        Ok(Some(serde_pyobject::to_pyobject(py, data)?.into()))
    } else {
        Ok(None)
    }
}

/// Get the current job board as a list of job dicts (eid, priority, state, ...).
pub fn get_job_board(pyworld: &PyWorld, py: Python) -> PyResult<PyObject> {
    let mut world = pyworld.inner.borrow_mut();
    // Using a raw pointer for interior mutability as in original
    let world_ptr: *mut _ = &mut *world;
    unsafe {
        world.job_board.update(&*world_ptr, 0, &[]);
        let entries = world.job_board.jobs_with_metadata(&*world_ptr);
        let py_entries = PyList::empty(py);
        for entry in entries {
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("eid", entry.eid)?;
            dict.set_item("priority", entry.priority)?;
            dict.set_item("state", entry.state)?;
            py_entries.append(dict)?;
        }
        Ok(py_entries.into())
    }
}

/// Get the current job board scheduling policy as a string.
pub fn get_job_board_policy(pyworld: &PyWorld) -> String {
    let world = pyworld.inner.borrow();
    world.job_board.get_policy_name().to_string()
}

/// Set the job board scheduling policy ("priority", "fifo", "lifo").
pub fn set_job_board_policy(pyworld: &PyWorld, policy: String) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    world
        .job_board
        .set_policy(&policy)
        .map_err(PyValueError::new_err)?;
    Ok(())
}

/// Get the priority value for a job by ID.
pub fn get_job_priority(pyworld: &PyWorld, job_id: u32) -> Option<i64> {
    let world = pyworld.inner.borrow();
    world.job_board.get_priority(&world, job_id)
}

/// Set the priority for a job by ID.
pub fn set_job_priority(pyworld: &PyWorld, job_id: u32, value: i64) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    let world_ptr: *mut _ = &mut *world;
    unsafe {
        world
            .job_board
            .set_priority(&mut *world_ptr, job_id, value)
            .map_err(PyValueError::new_err)?;
    }
    Ok(())
}
