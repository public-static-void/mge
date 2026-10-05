//! Treaty trade operations for [`WasmWorld`](super::wasm::WasmWorld).
//!
//! Thin mirror of [`crate::trade`]: validation, atomicity, epsilon discipline,
//! and gate order live in one place per world type, so the WASM bridge observes
//! behavior identical to the Lua/Python bridges (which call into
//! [`crate::trade`] directly — impossible here because `WasmWorld` stores its
//! own component and diplomacy state). Error strings prefix the core variant
//! name so guest-side assertions match on it, exactly like the script bridges.

use super::wasm::WasmWorld;
use crate::diplomacy::{RelationState, TreatyKind, TreatyStatus};
use crate::trade::TRANSFER_EPSILON;

impl WasmWorld {
    /// Reads one resource balance from an entity's stockpile.
    ///
    /// Returns `None` when the entity carries no `Stockpile` component; a
    /// missing kind entry reads as `0.0`.
    fn trade_balance(&self, entity: u32, kind: &str) -> Option<f64> {
        self.components
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
    fn set_trade_balance(&mut self, entity: u32, kind: &str, amount: f64) {
        if let Some(stockpile) = self
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

    /// Reads an entity's faction id from its `Faction` component.
    fn trade_entity_faction(&self, entity: u32) -> Option<String> {
        self.components
            .get("Faction")
            .and_then(|map| map.get(&entity))
            .and_then(|faction| faction.get("faction_id"))
            .and_then(|id| id.as_str())
            .filter(|id| !id.is_empty())
            .map(|id| id.to_string())
    }

    /// Moves `amount` of `kind` from one entity's stockpile to another's.
    ///
    /// Mirrors [`crate::trade::transfer_stockpile_resource`]: validation runs
    /// before any mutation, so every `Err` leaves both stockpiles
    /// byte-identical. A transfer to self is a validated no-op returning `Ok`.
    /// Shortfalls within [`TRANSFER_EPSILON`] succeed with the source floored
    /// at zero. Runs in O(1): two direct component lookups, no world scan.
    pub fn transfer_stockpile_resource(
        &mut self,
        from: u32,
        to: u32,
        kind: &str,
        amount: f64,
    ) -> Result<(), String> {
        if kind.is_empty() {
            return Err(format!("UnknownKind: unknown resource kind '{kind}'"));
        }
        if !amount.is_finite() || amount <= 0.0 {
            return Err(
                "NonPositiveAmount: transfer amount must be positive and finite".to_string(),
            );
        }
        if from == to {
            return Ok(());
        }
        let available = self
            .trade_balance(from, kind)
            .ok_or_else(|| format!("NoStockpile: entity {from} has no Stockpile component"))?;
        self.trade_balance(to, kind)
            .ok_or_else(|| format!("NoStockpile: entity {to} has no Stockpile component"))?;
        if available + TRANSFER_EPSILON < amount {
            return Err(format!(
                "InsufficientFunds: insufficient {kind}: required {amount}, available {available}"
            ));
        }
        let remainder = available - amount;
        self.set_trade_balance(
            from,
            kind,
            if remainder.abs() <= TRANSFER_EPSILON {
                0.0
            } else {
                remainder
            },
        );
        let dest_balance = self.trade_balance(to, kind).unwrap_or(0.0);
        self.set_trade_balance(to, kind, dest_balance + amount);
        Ok(())
    }

    /// Reports whether an accepted trade treaty binds two factions.
    ///
    /// Mirrors [`crate::trade::has_active_trade_treaty`]: matches `Active`
    /// `TradeStub` records on the canonical pair in either party order.
    /// Proposed, broken, and expired treaties do not count.
    pub fn has_active_trade_treaty(&self, faction_a: &str, faction_b: &str) -> bool {
        self.diplomacy.query_treaties(None).iter().any(|t| {
            t.kind == TreatyKind::TradeStub
                && t.status == TreatyStatus::Active
                && ((t.a == faction_a && t.b == faction_b)
                    || (t.a == faction_b && t.b == faction_a))
        })
    }

    /// Executes a treaty-gated transfer between two entities' stockpiles.
    ///
    /// Mirrors [`crate::trade::execute_treaty_trade`]: factions derive from
    /// the endpoints' `Faction` components, then a live accepted `TradeStub`
    /// treaty and a non-war relation are required before the transfer runs.
    /// Gate checks precede transfer validation, and every `Err` leaves both
    /// stockpiles byte-identical.
    pub fn execute_treaty_trade(
        &mut self,
        from: u32,
        to: u32,
        kind: &str,
        amount: f64,
    ) -> Result<(), String> {
        let Some(faction_from) = self.trade_entity_faction(from) else {
            return Err(
                "NoLiveTreaty: no live trade treaty between the parties' factions".to_string(),
            );
        };
        let Some(faction_to) = self.trade_entity_faction(to) else {
            return Err(
                "NoLiveTreaty: no live trade treaty between the parties' factions".to_string(),
            );
        };
        if !self.has_active_trade_treaty(&faction_from, &faction_to) {
            return Err(
                "NoLiveTreaty: no live trade treaty between the parties' factions".to_string(),
            );
        }
        if self.diplomacy.query_relation(&faction_from, &faction_to) == RelationState::War {
            return Err("RelationIsWar: treaty trade blocked: the parties are at war".to_string());
        }
        self.transfer_stockpile_resource(from, to, kind, amount)
    }
}
