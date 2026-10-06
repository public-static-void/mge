//! Supply API: create_supply_link, remove_supply_link, list_supply_links,
//! set_supply_link_active, get_supply_link.
//!
//! Thin delegation over [`engine_core::supply`]: validation, atomicity, epsilon
//! discipline, and gate order live in core, so every bridge observes identical
//! behavior. Errors surface as `ValueError` (the existing `PyWorld` error
//! convention) with the core variant name prefixed, since core `Display` text
//! omits it and script-side assertions match on it.

use crate::python_api::world::PyWorld;
use engine_core::supply::{self, SupplyError};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Variant token for a supply failure, surfaced in the script error text.
fn supply_variant(err: &SupplyError) -> &'static str {
    match err {
        SupplyError::SameEndpoint => "SameEndpoint",
        SupplyError::UnknownKind(_) => "UnknownKind",
        SupplyError::NonPositiveAmount => "NonPositiveAmount",
        SupplyError::NoStockpile(_) => "NoStockpile",
        SupplyError::UnknownLink(_) => "UnknownLink",
        SupplyError::Store(_) => "Store",
    }
}

/// Creates a supply link between two stockpiled entities and returns the link
/// entity id. New links start active.
pub fn create_supply_link(
    pyworld: &PyWorld,
    source: u32,
    target: u32,
    kind: String,
    amount_per_tick: f64,
    capacity_per_tick: f64,
) -> PyResult<u32> {
    let mut world = pyworld.inner.borrow_mut();
    supply::create_supply_link(
        &mut world,
        source,
        target,
        &kind,
        amount_per_tick,
        capacity_per_tick,
    )
    .map_err(|e| PyValueError::new_err(format!("{}: {e}", supply_variant(&e))))
}

/// Removes a link's `SupplyLink` component. Errors on unknown links.
pub fn remove_supply_link(pyworld: &PyWorld, link: u32) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    supply::remove_supply_link(&mut world, link)
        .map_err(|e| PyValueError::new_err(format!("{}: {e}", supply_variant(&e))))
}

/// Lists link entity ids in ascending order.
pub fn list_supply_links(pyworld: &PyWorld) -> Vec<u32> {
    let world = pyworld.inner.borrow();
    supply::list_supply_links(&world)
}

/// Toggles a link's `active` flag. Errors on unknown links.
pub fn set_supply_link_active(pyworld: &PyWorld, link: u32, active: bool) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    supply::set_supply_link_active(&mut world, link, active)
        .map_err(|e| PyValueError::new_err(format!("{}: {e}", supply_variant(&e))))
}

/// Reads a link record as a dict with
/// `{source, target, kind, amount_per_tick, capacity_per_tick, active}`.
/// Raises `ValueError` on unknown links.
pub fn get_supply_link(pyworld: &PyWorld, py: Python<'_>, link: u32) -> PyResult<crate::PyObject> {
    let world = pyworld.inner.borrow();
    let record = supply::get_supply_link(&world, link).ok_or_else(|| {
        PyValueError::new_err(format!("UnknownLink: {}", SupplyError::UnknownLink(link)))
    })?;
    let value = serde_json::json!({
        "source": record.source,
        "target": record.target,
        "kind": record.kind,
        "amount_per_tick": record.amount_per_tick,
        "capacity_per_tick": record.capacity_per_tick,
        "active": record.active,
    });
    serde_pyobject::to_pyobject(py, &value)
        .map(|bound| bound.into())
        .map_err(|e| PyValueError::new_err(e.to_string()))
}
