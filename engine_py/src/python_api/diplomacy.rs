use crate::python_api::world::PyWorld;
use engine_core::diplomacy::{self, TreatyKind};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Parses a bridge kind string into the closed core kind set.
fn parse_kind(kind: &str) -> PyResult<TreatyKind> {
    TreatyKind::parse(kind).map_err(PyValueError::new_err)
}

/// Returns the canonical relation name for a faction pair.
pub fn get_relation(pyworld: &PyWorld, fa: String, fb: String) -> String {
    let world = pyworld.inner.borrow();
    diplomacy::get_relation(&world, &fa, &fb)
        .as_str()
        .to_string()
}

/// Returns the standing score for a faction pair.
pub fn get_standing(pyworld: &PyWorld, fa: String, fb: String) -> i64 {
    let world = pyworld.inner.borrow();
    diplomacy::get_standing(&world, &fa, &fb)
}

/// Applies a standing delta; bounds and validation live in core.
pub fn modify_standing(pyworld: &PyWorld, fa: String, fb: String, delta: i64) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    diplomacy::modify_standing(&mut world, &fa, &fb, delta).map_err(PyValueError::new_err)
}

/// Declares war between two factions.
pub fn declare_war(pyworld: &PyWorld, fa: String, fb: String) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    diplomacy::declare_war(&mut world, &fa, &fb).map_err(PyValueError::new_err)
}

/// Declares peace between two warring factions.
pub fn declare_peace(pyworld: &PyWorld, fa: String, fb: String) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    diplomacy::declare_peace(&mut world, &fa, &fb).map_err(PyValueError::new_err)
}

/// Proposes a treaty and returns the new treaty id.
/// `duration_ticks` of None means the treaty never expires.
pub fn propose_treaty(
    pyworld: &PyWorld,
    proposer: String,
    other: String,
    kind: String,
    duration_ticks: Option<u64>,
) -> PyResult<u64> {
    let kind = parse_kind(&kind)?;
    let mut world = pyworld.inner.borrow_mut();
    diplomacy::propose_treaty(&mut world, &proposer, &other, kind, duration_ticks)
        .map_err(PyValueError::new_err)
}

/// Accepts a proposed treaty.
pub fn accept_treaty(pyworld: &PyWorld, treaty_id: u64) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    diplomacy::accept_treaty(&mut world, treaty_id).map_err(PyValueError::new_err)
}

/// Breaks a proposed or active treaty.
pub fn break_treaty(pyworld: &PyWorld, treaty_id: u64) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    diplomacy::break_treaty(&mut world, treaty_id).map_err(PyValueError::new_err)
}

/// Lists treaty records as a list of dicts, optionally filtered to one faction.
pub fn list_treaties(
    pyworld: &PyWorld,
    py: Python<'_>,
    faction: Option<String>,
) -> PyResult<crate::PyObject> {
    let world = pyworld.inner.borrow();
    let records: Vec<serde_json::Value> = diplomacy::list_treaties(&world, faction.as_deref())
        .iter()
        .map(|treaty| treaty.to_json())
        .collect();
    let value = serde_json::Value::Array(records);
    serde_pyobject::to_pyobject(py, &value)
        .map(|bound| bound.into())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}
