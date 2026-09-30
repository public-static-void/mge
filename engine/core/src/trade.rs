//! Atomic inter-stockpile trade primitives and read-only treaty queries.
//!
//! [`transfer_stockpile_resource`] moves one resource kind between two
//! entities' `Stockpile.resources` maps in a single validated step: every
//! failure leaves both stockpiles byte-identical. [`has_active_trade_treaty`]
//! reports whether an accepted `TradeStub` treaty binds two factions without
//! touching the diplomacy store.

use crate::diplomacy::{TreatyKind, TreatyStatus};
use crate::ecs::world::World;

/// Tolerance for the insufficient-funds comparison and dust flooring.
///
/// A shortfall at or below this magnitude counts as paid in full, and a
/// remaining balance at or below it is floored to exactly zero so stockpiles
/// never hold negative dust.
pub const TRANSFER_EPSILON: f64 = 1e-9;

/// Failure modes for [`transfer_stockpile_resource`].
#[derive(Debug, Clone, PartialEq)]
pub enum TransferError {
    /// Either endpoint carries no `Stockpile` component (holds the entity).
    NoStockpile(u32),
    /// The resource kind is empty (holds the rejected kind).
    UnknownKind(String),
    /// The amount is zero, negative, NaN, or infinite.
    NonPositiveAmount,
    /// The source balance cannot cover the amount; nothing was moved.
    InsufficientFunds {
        /// Requested resource kind.
        kind: String,
        /// Requested amount.
        required: f64,
        /// Source balance at call time.
        available: f64,
    },
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferError::NoStockpile(entity) => {
                write!(f, "entity {entity} has no Stockpile component")
            }
            TransferError::UnknownKind(kind) => {
                write!(f, "unknown resource kind '{kind}'")
            }
            TransferError::NonPositiveAmount => {
                write!(f, "transfer amount must be positive and finite")
            }
            TransferError::InsufficientFunds {
                kind,
                required,
                available,
            } => write!(
                f,
                "insufficient {kind}: required {required}, available {available}"
            ),
        }
    }
}

impl std::error::Error for TransferError {}

/// Reads one resource balance from an entity's stockpile.
///
/// Returns `None` when the entity carries no `Stockpile` component; a missing
/// kind entry reads as `0.0`.
fn stockpile_balance(world: &World, entity: u32, kind: &str) -> Option<f64> {
    world
        .components
        .get("Stockpile")
        .and_then(|map| map.get(&entity))
        .map(|stockpile| {
            stockpile
                .get("resources")
                .and_then(|r| r.get(kind))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
        })
}

/// Writes one resource balance into an entity's stockpile.
///
/// The caller guarantees the entity carries a `Stockpile` component.
fn set_stockpile_balance(world: &mut World, entity: u32, kind: &str, amount: f64) {
    if let Some(stockpile) = world
        .components
        .get_mut("Stockpile")
        .and_then(|map| map.get_mut(&entity))
        && let Some(obj) = stockpile.as_object_mut()
    {
        let resources = obj
            .entry("resources")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(map) = resources.as_object_mut() {
            map.insert(kind.to_string(), serde_json::json!(amount));
        }
    }
}

/// Moves `amount` of `kind` from one entity's stockpile to another's.
///
/// Validation runs before any mutation, so every `Err` leaves both stockpiles
/// byte-identical. A transfer to self is a validated no-op returning `Ok`.
/// Shortfalls within [`TRANSFER_EPSILON`] succeed with the source floored at
/// zero. Runs in O(1): two direct component lookups, no world scan.
pub fn transfer_stockpile_resource(
    world: &mut World,
    from: u32,
    to: u32,
    kind: &str,
    amount: f64,
) -> Result<(), TransferError> {
    if kind.is_empty() {
        return Err(TransferError::UnknownKind(kind.to_string()));
    }
    if !amount.is_finite() || amount <= 0.0 {
        return Err(TransferError::NonPositiveAmount);
    }
    if from == to {
        return Ok(());
    }
    let available = stockpile_balance(world, from, kind).ok_or(TransferError::NoStockpile(from))?;
    stockpile_balance(world, to, kind).ok_or(TransferError::NoStockpile(to))?;
    if available + TRANSFER_EPSILON < amount {
        return Err(TransferError::InsufficientFunds {
            kind: kind.to_string(),
            required: amount,
            available,
        });
    }
    let remainder = available - amount;
    set_stockpile_balance(
        world,
        from,
        kind,
        if remainder.abs() <= TRANSFER_EPSILON {
            0.0
        } else {
            remainder
        },
    );
    let dest_balance = stockpile_balance(world, to, kind).unwrap_or(0.0);
    set_stockpile_balance(world, to, kind, dest_balance + amount);
    Ok(())
}

/// Reports whether an accepted trade treaty binds two factions.
///
/// Matches `Active` `TradeStub` records on the canonical pair in either party
/// order. Proposed, broken, and expired treaties do not count. Read-only over
/// the diplomacy store: takes `&World` and performs zero writes.
pub fn has_active_trade_treaty(world: &World, faction_a: &str, faction_b: &str) -> bool {
    world.diplomacy.query_treaties(None).iter().any(|t| {
        t.kind == TreatyKind::TradeStub
            && t.status == TreatyStatus::Active
            && ((t.a == faction_a && t.b == faction_b) || (t.a == faction_b && t.b == faction_a))
    })
}
