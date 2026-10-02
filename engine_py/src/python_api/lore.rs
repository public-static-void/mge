//! Lore API: generate_founding_history, list_chronicle,
//! get_chronicle_entry, render_chronicle, chronicle_len,
//! clear_lore_history.
//!
//! Thin delegation over [`engine_core::lore`]: backfill generation and the
//! chronicle read-model live in core, so every bridge observes identical
//! behavior. Payload shapes serialize from the same core types, keeping the
//! Lua/Python/WASM surfaces byte-compatible.

use crate::python_api::world::PyWorld;
use engine_core::lore::{self, ChronicleFilter, ChronicleKind};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Parses a lowercase kind token into its chronicle kind.
///
/// Rejects anything outside the fixed `fired|resolved|expired|founding` set
/// so invalid filters raise instead of silently matching nothing.
fn parse_kind(raw: &str) -> PyResult<ChronicleKind> {
    match raw {
        "fired" => Ok(ChronicleKind::Fired),
        "resolved" => Ok(ChronicleKind::Resolved),
        "expired" => Ok(ChronicleKind::Expired),
        "founding" => Ok(ChronicleKind::Founding),
        other => Err(PyValueError::new_err(format!(
            "Invalid chronicle kind: {other}"
        ))),
    }
}

/// Parses an optional filter dict into its core filter.
///
/// A missing filter means unbounded; each missing key stays unbounded.
/// Mirrors the Lua filter-table and WASM JSON-filter shapes key for key.
fn parse_filter(filter: Option<Bound<'_, PyAny>>) -> PyResult<ChronicleFilter> {
    let mut out = ChronicleFilter::default();
    let Some(bound) = filter else {
        return Ok(out);
    };
    let value: serde_json::Value =
        serde_pyobject::from_pyobject(bound).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let obj = value
        .as_object()
        .ok_or_else(|| PyValueError::new_err("Chronicle filter must be a dict"))?;
    if let Some(scenario) = obj.get("scenario_id")
        && !scenario.is_null()
    {
        out.scenario_id = Some(
            scenario
                .as_str()
                .ok_or_else(|| {
                    PyValueError::new_err("Chronicle filter scenario_id must be a string")
                })?
                .to_string(),
        );
    }
    if let Some(kind) = obj.get("kind")
        && !kind.is_null()
    {
        let raw = kind
            .as_str()
            .ok_or_else(|| PyValueError::new_err("Chronicle filter kind must be a string"))?;
        out.kind = Some(parse_kind(raw)?);
    }
    if let Some(from) = obj.get("turn_from")
        && !from.is_null()
    {
        out.turn_from = Some(from.as_u64().ok_or_else(|| {
            PyValueError::new_err("Chronicle filter turn_from must be a non-negative integer")
        })?);
    }
    if let Some(to) = obj.get("turn_to")
        && !to.is_null()
    {
        out.turn_to = Some(to.as_u64().ok_or_else(|| {
            PyValueError::new_err("Chronicle filter turn_to must be a non-negative integer")
        })?);
    }
    Ok(out)
}

/// Appends seeded founding-era entries and returns the appended count.
///
/// Raises `ValueError` on negative inputs or core backfill failures,
/// leaving prior lore state untouched.
pub fn generate_founding_history(pyworld: &PyWorld, seed: i64, era_count: i64) -> PyResult<usize> {
    let seed_value = u64::try_from(seed)
        .map_err(|_| PyValueError::new_err(format!("Invalid lore seed: {seed}")))?;
    let era_value = u32::try_from(era_count)
        .map_err(|_| PyValueError::new_err(format!("Invalid era count: {era_count}")))?;
    let mut world = pyworld.inner.borrow_mut();
    lore::generate_founding_history(&mut world, seed_value, era_value)
        .map_err(PyValueError::new_err)
}

/// Lists chronicle entries matching the filter in (turn, entry id) order.
///
/// A missing filter returns every entry. Raises `ValueError` on invalid
/// filter values.
pub fn list_chronicle(
    pyworld: &PyWorld,
    py: Python<'_>,
    filter: Option<Bound<'_, PyAny>>,
) -> PyResult<crate::PyObject> {
    let world = pyworld.inner.borrow();
    let entries = lore::list_chronicle(&world, parse_filter(filter)?);
    let value = serde_json::to_value(entries).unwrap_or_default();
    serde_pyobject::to_pyobject(py, &value)
        .map(|bound| bound.into())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Returns the chronicle entry for an id as a dict, or None.
///
/// Unknown ids return None and never raise, matching the bridge contract.
pub fn get_chronicle_entry(
    pyworld: &PyWorld,
    py: Python<'_>,
    entry_id: i64,
) -> PyResult<Option<crate::PyObject>> {
    let world = pyworld.inner.borrow();
    let id = u64::try_from(entry_id).unwrap_or(u64::MAX);
    match lore::get_chronicle_entry(&world, id) {
        Some(entry) => {
            let value = serde_json::to_value(entry).unwrap_or_default();
            serde_pyobject::to_pyobject(py, &value)
                .map(|bound| Some(bound.into()))
                .map_err(|e| PyValueError::new_err(e.to_string()))
        }
        None => Ok(None),
    }
}

/// Renders the filtered chronicle as human-readable template lines.
///
/// Raises `ValueError` on invalid filter values.
pub fn render_chronicle(
    pyworld: &PyWorld,
    py: Python<'_>,
    filter: Option<Bound<'_, PyAny>>,
) -> PyResult<crate::PyObject> {
    let world = pyworld.inner.borrow();
    let lines = lore::render_chronicle_text(&world, parse_filter(filter)?);
    let value = serde_json::to_value(lines).unwrap_or_default();
    serde_pyobject::to_pyobject(py, &value)
        .map(|bound| bound.into())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Returns the number of chronicle entries on the world.
pub fn chronicle_len(pyworld: &PyWorld) -> usize {
    let world = pyworld.inner.borrow();
    lore::chronicle_len(&world)
}

/// Clears all chronicle entries and resets the id counter.
///
/// Test and regen support: call before re-running seeded backfill
/// generation to reproduce byte-identical chronicles.
pub fn clear_lore_history(pyworld: &PyWorld) {
    let mut world = pyworld.inner.borrow_mut();
    lore::clear_lore_history(&mut world);
}
