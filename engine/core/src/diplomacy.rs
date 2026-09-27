//! World-level diplomacy store: faction-pair relationships.
//!
//! Holds bilateral state between faction ids (not entities) keyed by the
//! canonical `(min, max)` pair, following the free-function accessor split of
//! `faction.rs` / `tech_tree.rs`: module-level state on [`World`] plus thin
//! accessors, with no per-entity owner required.
//!
//! Relationship state is explicit-only: it changes solely through war, peace,
//! and treaty transitions (M4/M3), never derived from entity `Reputation`
//! scores or `Faction.role`. Every successful mutation emits exactly one
//! `relation_changed` event; rejected mutations emit nothing.

use crate::ecs::world::World;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// Minimum standing score for a faction pair.
pub const MIN_STANDING: i64 = -100;
/// Maximum standing score for a faction pair.
pub const MAX_STANDING: i64 = 100;

/// Bilateral relationship state between two factions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RelationState {
    /// Formal allies (set by Alliance treaty acceptance).
    Allied,
    /// Default state for unknown pairs.
    #[default]
    Neutral,
    /// Openly antagonistic short of declared war.
    Hostile,
    /// Declared war (set by `declare_war`, cleared by `declare_peace`).
    War,
}

/// Standing and state for one canonical faction pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationEntry {
    /// Explicit relationship state (never implicitly derived).
    pub state: RelationState,
    /// Diplomatic standing, always within `MIN_STANDING..=MAX_STANDING`.
    pub standing: i64,
}

impl Default for RelationEntry {
    fn default() -> Self {
        Self {
            state: RelationState::Neutral,
            standing: 0,
        }
    }
}

/// World-level diplomacy store, held on [`World`] and serialized for
/// whole-world save/load.
///
/// Pair keys are stored nested (`outer = min(fa, fb)`, `inner = max(fa, fb)`):
/// nested string keys serialize cleanly to JSON, which tuple keys cannot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiplomacyState {
    pairs: HashMap<String, HashMap<String, RelationEntry>>,
}

impl DiplomacyState {
    /// Returns the canonical `(min, max)` key for a faction pair.
    fn canonical(fa: &str, fb: &str) -> (String, String) {
        if fa <= fb {
            (fa.to_string(), fb.to_string())
        } else {
            (fb.to_string(), fa.to_string())
        }
    }

    /// Returns the entry for a canonical pair, or the neutral default.
    fn get(&self, fa: &str, fb: &str) -> RelationEntry {
        let (a, b) = Self::canonical(fa, fb);
        self.pairs
            .get(&a)
            .and_then(|inner| inner.get(&b))
            .copied()
            .unwrap_or_default()
    }

    /// Returns a mutable reference to a pair entry, creating it on demand.
    fn get_or_create(&mut self, fa: &str, fb: &str) -> &mut RelationEntry {
        let (a, b) = Self::canonical(fa, fb);
        self.pairs.entry(a).or_default().entry(b).or_default()
    }
}

/// Validates faction ids for a mutating diplomacy operation.
///
/// Rejects self-diplomacy (`fa == fb`) and empty ids. Emits no event.
pub fn can_modify_standing(fa: &str, fb: &str) -> Result<(), String> {
    if fa == fb {
        return Err(format!("self-diplomacy not allowed for '{fa}'"));
    }
    if fa.is_empty() || fb.is_empty() {
        return Err("faction ids must be non-empty".to_string());
    }
    Ok(())
}

/// Returns the relationship state for a faction pair.
///
/// Unknown pairs (and self-pairs) report [`RelationState::Neutral`].
/// Lookup is order-independent: `(fa, fb)` equals `(fb, fa)`.
pub fn get_relation(world: &World, fa: &str, fb: &str) -> RelationState {
    world.diplomacy.get(fa, fb).state
}

/// Returns the standing score for a faction pair.
///
/// Unknown pairs (and self-pairs) report `0`. Lookup is order-independent.
pub fn get_standing(world: &World, fa: &str, fb: &str) -> i64 {
    world.diplomacy.get(fa, fb).standing
}

/// Applies a symmetric standing delta to a faction pair, clamped to
/// `MIN_STANDING..=MAX_STANDING`, and emits exactly one `relation_changed`
/// event carrying `{ a, b, old_standing, new_standing, state }` in canonical
/// pair order.
///
/// Relationship state is preserved: standing never promotes or demotes state
/// implicitly. Self-pair and empty-id calls return `Err` and emit no event.
pub fn modify_standing(world: &mut World, fa: &str, fb: &str, delta: i64) -> Result<(), String> {
    can_modify_standing(fa, fb)?;
    let (a, b) = DiplomacyState::canonical(fa, fb);
    let entry = world.diplomacy.get_or_create(&a, &b);
    let old_standing = entry.standing;
    let new_standing = (old_standing + delta).clamp(MIN_STANDING, MAX_STANDING);
    entry.standing = new_standing;
    let state = entry.state;
    world.send_event(
        "relation_changed",
        json!({
            "a": a,
            "b": b,
            "old_standing": old_standing,
            "new_standing": new_standing,
            "state": state,
        }),
    )?;
    Ok(())
}
