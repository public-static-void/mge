use crate::python_api::world::PyWorld;
use engine_core::narrative;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Registers a scenario definition from its JSON encoding.
///
/// Raises `ValueError` on malformed JSON or invalid definitions, leaving
/// prior narrative state untouched.
pub fn register_scenario(pyworld: &PyWorld, def_json: String) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    narrative::register_scenario(&mut world, &def_json).map_err(PyValueError::new_err)
}

/// Lists registered scenario definitions in id order as a list of dicts.
pub fn list_scenarios(pyworld: &PyWorld, py: Python<'_>) -> PyResult<crate::PyObject> {
    let world = pyworld.inner.borrow();
    let defs = narrative::list_scenarios(&world);
    let value = serde_json::to_value(defs).unwrap_or_default();
    serde_pyobject::to_pyobject(py, &value)
        .map(|bound| bound.into())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Returns the scenario definition for an id as a dict, or None.
pub fn get_scenario(
    pyworld: &PyWorld,
    py: Python<'_>,
    id: String,
) -> PyResult<Option<crate::PyObject>> {
    let world = pyworld.inner.borrow();
    match narrative::get_scenario(&world, &id) {
        Some(def) => {
            let value = serde_json::to_value(def).unwrap_or_default();
            serde_pyobject::to_pyobject(py, &value)
                .map(|bound| Some(bound.into()))
                .map_err(|e| PyValueError::new_err(e.to_string()))
        }
        None => Ok(None),
    }
}

/// Lists live pending decisions in id order as a list of dicts.
pub fn poll_pending_decisions(pyworld: &PyWorld, py: Python<'_>) -> PyResult<crate::PyObject> {
    let world = pyworld.inner.borrow();
    let pending = narrative::list_pending_decisions(&world);
    let value = serde_json::to_value(pending).unwrap_or_default();
    serde_pyobject::to_pyobject(py, &value)
        .map(|bound| bound.into())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Returns the live pending decision for an id as a dict, or None.
pub fn get_pending_decision(
    pyworld: &PyWorld,
    py: Python<'_>,
    decision_id: u64,
) -> PyResult<Option<crate::PyObject>> {
    let world = pyworld.inner.borrow();
    match narrative::get_pending_decision(&world, decision_id) {
        Some(decision) => {
            let value = serde_json::to_value(decision).unwrap_or_default();
            serde_pyobject::to_pyobject(py, &value)
                .map(|bound| Some(bound.into()))
                .map_err(|e| PyValueError::new_err(e.to_string()))
        }
        None => Ok(None),
    }
}

/// Resolves a pending decision with one of its choices.
///
/// Applies the choice's effects in order and records history.
/// Raises `ValueError` on unknown or settled decisions and unknown choices,
/// with no state change.
pub fn resolve_decision(pyworld: &PyWorld, decision_id: u64, choice_id: String) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    narrative::resolve_decision(&mut world, decision_id, &choice_id).map_err(PyValueError::new_err)
}

/// Returns the turn-indexed narrative history in append order as a list of dicts.
pub fn get_narrative_history(pyworld: &PyWorld, py: Python<'_>) -> PyResult<crate::PyObject> {
    let world = pyworld.inner.borrow();
    let history = narrative::get_narrative_history(&world);
    let value = serde_json::to_value(history).unwrap_or_default();
    serde_pyobject::to_pyobject(py, &value)
        .map(|bound| bound.into())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}
