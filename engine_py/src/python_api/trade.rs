//! Trade API: transfer_stockpile_resource, has_active_trade_treaty,
//! execute_treaty_trade.
//!
//! Thin delegation over [`engine_core::trade`]: validation, atomicity, epsilon
//! discipline, and gate order live in core, so every bridge observes identical
//! behavior. Errors surface as `ValueError` (the existing `PyWorld` error
//! convention) with the core variant name prefixed, since core `Display` text
//! omits it and script-side assertions match on it.

use crate::python_api::world::PyWorld;
use engine_core::trade::{self, TradeError, TransferError};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Variant token for a transfer failure, surfaced in the script error text.
fn transfer_variant(err: &TransferError) -> &'static str {
    match err {
        TransferError::NoStockpile(_) => "NoStockpile",
        TransferError::UnknownKind(_) => "UnknownKind",
        TransferError::NonPositiveAmount => "NonPositiveAmount",
        TransferError::InsufficientFunds { .. } => "InsufficientFunds",
    }
}

/// Variant token for a treaty-trade failure, surfaced in the script error text.
fn trade_variant(err: &TradeError) -> &'static str {
    match err {
        TradeError::Transfer(inner) => transfer_variant(inner),
        TradeError::NoLiveTreaty => "NoLiveTreaty",
        TradeError::RelationIsWar => "RelationIsWar",
    }
}

/// Moves `amount` of `kind` between two entities' stockpiles. Ungated
/// scripter-trust move, like `modify_stockpile_resource`.
pub fn transfer_stockpile_resource(
    pyworld: &PyWorld,
    from_id: u32,
    to_id: u32,
    kind: String,
    amount: f64,
) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    trade::transfer_stockpile_resource(&mut world, from_id, to_id, &kind, amount)
        .map_err(|e| PyValueError::new_err(format!("{}: {e}", transfer_variant(&e))))
}

/// Reports whether an accepted trade treaty binds two factions.
pub fn has_active_trade_treaty(pyworld: &PyWorld, faction_a: String, faction_b: String) -> bool {
    let world = pyworld.inner.borrow();
    trade::has_active_trade_treaty(&world, &faction_a, &faction_b)
}

/// Treaty- and peace-gated transfer between two entities' stockpiles.
pub fn execute_treaty_trade(
    pyworld: &PyWorld,
    from_id: u32,
    to_id: u32,
    kind: String,
    amount: f64,
) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    trade::execute_treaty_trade(&mut world, from_id, to_id, &kind, amount)
        .map_err(|e| PyValueError::new_err(format!("{}: {e}", trade_variant(&e))))
}
