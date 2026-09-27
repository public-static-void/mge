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
    /// All treaty records by monotonic id. Defaults empty for pre-treaty saves.
    #[serde(default)]
    treaties: HashMap<u64, Treaty>,
    /// Next treaty id counter. Defaults zero for pre-treaty saves.
    #[serde(default)]
    next_treaty_id: u64,
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

/// Standing penalty applied to a pair when one of its treaties is broken.
pub const TREATY_BREAK_PENALTY: i64 = -25;

/// Kind of bilateral treaty between two factions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreatyKind {
    /// Mutual pledge not to attack; acceptance floors a hostile pair to Neutral.
    NonAggression,
    /// Formal alliance; acceptance raises the pair to Allied.
    Alliance,
    /// Ends a war; acceptance floors a hostile pair to Neutral.
    Peace,
    /// Economic placeholder: runs the full lifecycle with no pair-state effect.
    TradeStub,
}

/// Lifecycle status of a treaty record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreatyStatus {
    /// Proposed but not yet accepted.
    Proposed,
    /// Accepted and in force (subject to expiry).
    Active,
    /// Ended early by `break_treaty`.
    Broken,
    /// Ended by reaching `expires_tick`.
    Expired,
}

/// One bilateral treaty record.
///
/// Party order (`a`, `b`) is preserved as proposed; pair lookups on the record
/// use the canonical `(min, max)` order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Treaty {
    /// Monotonic id from the store counter.
    pub id: u64,
    /// Proposing faction, as passed to `propose_treaty`.
    pub a: String,
    /// Counterparty faction, as passed to `propose_treaty`.
    pub b: String,
    /// Treaty kind.
    pub kind: TreatyKind,
    /// Current lifecycle status.
    pub status: TreatyStatus,
    /// Turn the treaty was proposed on.
    pub proposed_tick: u64,
    /// Turn the treaty was accepted on (`None` until accepted).
    pub active_tick: Option<u64>,
    /// Turn the treaty expires on (`None` means it never expires).
    pub expires_tick: Option<u64>,
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

/// Validates a treaty proposal.
///
/// Rejects self/empty ids, non-`Peace` kinds while the pair is at war, and a
/// duplicate same-kind treaty that is still `Proposed` or `Active`.
/// Emits no event.
pub fn can_propose(
    world: &World,
    proposer: &str,
    other: &str,
    kind: &TreatyKind,
) -> Result<(), String> {
    can_modify_standing(proposer, other)?;
    if get_relation(world, proposer, other) == RelationState::War && *kind != TreatyKind::Peace {
        return Err(format!(
            "cannot propose {kind:?} while '{proposer}' and '{other}' are at war"
        ));
    }
    let (a, b) = DiplomacyState::canonical(proposer, other);
    let duplicate = world.diplomacy.treaties.values().any(|t| {
        let (ta, tb) = DiplomacyState::canonical(&t.a, &t.b);
        ta == a && tb == b && t.kind == *kind && is_live(t.status)
    });
    if duplicate {
        return Err(format!(
            "duplicate live {kind:?} treaty between '{proposer}' and '{other}'"
        ));
    }
    Ok(())
}

/// Returns true while a treaty still binds its parties (`Proposed`/`Active`).
fn is_live(status: TreatyStatus) -> bool {
    matches!(status, TreatyStatus::Proposed | TreatyStatus::Active)
}

/// Proposes a treaty between two factions, returning the new treaty id.
///
/// Records `{ id, parties, kind, status: Proposed, proposed_tick, expires_tick }`
/// with party order preserved as passed, and emits exactly one
/// `treaty_proposed { treaty_id, proposer, other, kind }` event.
/// `duration_ticks: None` means the treaty never expires.
pub fn propose_treaty(
    world: &mut World,
    proposer: &str,
    other: &str,
    kind: TreatyKind,
    duration_ticks: Option<u64>,
) -> Result<u64, String> {
    can_propose(world, proposer, other, &kind)?;
    let turn = u64::from(world.turn);
    let id = world.diplomacy.next_treaty_id;
    world.diplomacy.next_treaty_id += 1;
    world.diplomacy.treaties.insert(
        id,
        Treaty {
            id,
            a: proposer.to_string(),
            b: other.to_string(),
            kind,
            status: TreatyStatus::Proposed,
            proposed_tick: turn,
            active_tick: None,
            expires_tick: duration_ticks.map(|d| turn + d),
        },
    );
    world.send_event(
        "treaty_proposed",
        json!({
            "treaty_id": id,
            "proposer": proposer,
            "other": other,
            "kind": kind,
        }),
    )?;
    Ok(id)
}

/// Validates a treaty acceptance: the record must exist and be `Proposed`.
/// Emits no event.
pub fn can_accept(world: &World, treaty_id: u64) -> Result<(), String> {
    match world.diplomacy.treaties.get(&treaty_id) {
        Some(t) if t.status == TreatyStatus::Proposed => Ok(()),
        Some(_) => Err(format!("treaty {treaty_id} is not proposed")),
        None => Err(format!("unknown treaty {treaty_id}")),
    }
}

/// Accepts a proposed treaty: sets `Active` with `active_tick = turn`, applies
/// the kind's state floor (`Peace`/`NonAggression` lift `Hostile` to `Neutral`,
/// `Alliance` raises to `Allied` unless at war, `TradeStub` changes nothing),
/// and emits exactly one `treaty_signed { treaty_id, kind }` event.
pub fn accept_treaty(world: &mut World, treaty_id: u64) -> Result<(), String> {
    can_accept(world, treaty_id)?;
    let turn = u64::from(world.turn);
    let treaty = world
        .diplomacy
        .treaties
        .get_mut(&treaty_id)
        .expect("treaty validated by can_accept");
    treaty.status = TreatyStatus::Active;
    treaty.active_tick = Some(turn);
    let (kind, a, b) = (treaty.kind, treaty.a.clone(), treaty.b.clone());
    apply_accept_floor(world, kind, &a, &b);
    world.send_event(
        "treaty_signed",
        json!({
            "treaty_id": treaty_id,
            "kind": kind,
        }),
    )?;
    Ok(())
}

/// Applies a treaty kind's state floor to its pair on acceptance.
///
/// Floors never demote: `Allied` survives `Peace`/`NonAggression`, and `War`
/// is left intact (only peace transitions clear it).
fn apply_accept_floor(world: &mut World, kind: TreatyKind, fa: &str, fb: &str) {
    let (a, b) = DiplomacyState::canonical(fa, fb);
    let entry = world.diplomacy.get_or_create(&a, &b);
    match kind {
        TreatyKind::Peace | TreatyKind::NonAggression => {
            if entry.state == RelationState::Hostile {
                entry.state = RelationState::Neutral;
            }
        }
        TreatyKind::Alliance => {
            if entry.state != RelationState::War {
                entry.state = RelationState::Allied;
            }
        }
        TreatyKind::TradeStub => {}
    }
}

/// Validates breaking a treaty: the record must exist and be
/// `Proposed` or `Active`. Emits no event.
pub fn can_break(world: &World, treaty_id: u64) -> Result<(), String> {
    match world.diplomacy.treaties.get(&treaty_id) {
        Some(t) if is_live(t.status) => Ok(()),
        Some(_) => Err(format!("treaty {treaty_id} is already settled")),
        None => Err(format!("unknown treaty {treaty_id}")),
    }
}

/// Breaks a proposed or active treaty: sets `Broken`, applies the
/// `TREATY_BREAK_PENALTY` standing penalty (clamped), and emits exactly one
/// `treaty_broken { treaty_id, penalty }` event.
///
/// The penalty adjusts pair standing directly without a `relation_changed`
/// event so each transition emits exactly one event.
pub fn break_treaty(world: &mut World, treaty_id: u64) -> Result<(), String> {
    can_break(world, treaty_id)?;
    let treaty = world
        .diplomacy
        .treaties
        .get_mut(&treaty_id)
        .expect("treaty validated by can_break");
    treaty.status = TreatyStatus::Broken;
    let (a, b) = DiplomacyState::canonical(&treaty.a.clone(), &treaty.b.clone());
    let entry = world.diplomacy.get_or_create(&a, &b);
    entry.standing = (entry.standing + TREATY_BREAK_PENALTY).clamp(MIN_STANDING, MAX_STANDING);
    world.send_event(
        "treaty_broken",
        json!({
            "treaty_id": treaty_id,
            "penalty": TREATY_BREAK_PENALTY,
        }),
    )?;
    Ok(())
}

/// Expires every `Active` treaty whose `expires_tick` has reached the current
/// turn, emitting one `treaty_expired { treaty_id }` event per treaty.
///
/// Boundary-exact: `expires_tick <= turn` expires, `expires_tick > turn`
/// survives, and `expires_tick: None` never expires. Deterministic
/// collect-then-apply; no wall-clock or RNG.
pub fn expire_due_treaties(world: &mut World) -> Result<(), String> {
    let turn = u64::from(world.turn);
    let due: Vec<u64> = world
        .diplomacy
        .treaties
        .values()
        .filter(|t| t.status == TreatyStatus::Active && t.expires_tick.is_some_and(|e| e <= turn))
        .map(|t| t.id)
        .collect();
    for id in due {
        if let Some(treaty) = world.diplomacy.treaties.get_mut(&id) {
            treaty.status = TreatyStatus::Expired;
        }
        world.send_event("treaty_expired", json!({ "treaty_id": id }))?;
    }
    Ok(())
}

/// Lists treaty records, optionally filtered to one faction's membership.
pub fn list_treaties(world: &World, faction: Option<&str>) -> Vec<Treaty> {
    let mut out: Vec<Treaty> = world
        .diplomacy
        .treaties
        .values()
        .filter(|t| faction.is_none_or(|f| t.a == f || t.b == f))
        .cloned()
        .collect();
    out.sort_by_key(|t| t.id);
    out
}

/// Validates a war declaration: distinct non-empty ids, pair not already at war.
/// Emits no event.
pub fn can_war(world: &World, fa: &str, fb: &str) -> Result<(), String> {
    can_modify_standing(fa, fb)?;
    if get_relation(world, fa, fb) == RelationState::War {
        return Err(format!("'{fa}' and '{fb}' are already at war"));
    }
    Ok(())
}

/// Declares war between two factions: sets pair state to `War` and emits
/// exactly one `war_declared { aggressor, defender, tick }` event.
///
/// Minimal M3 transition: the war-breaks-paper auto-break of live treaties
/// lands in M4.
pub fn declare_war(world: &mut World, aggressor: &str, defender: &str) -> Result<(), String> {
    can_war(world, aggressor, defender)?;
    let (a, b) = DiplomacyState::canonical(aggressor, defender);
    world.diplomacy.get_or_create(&a, &b).state = RelationState::War;
    let tick = u64::from(world.turn);
    world.send_event(
        "war_declared",
        json!({
            "aggressor": aggressor,
            "defender": defender,
            "tick": tick,
        }),
    )?;
    Ok(())
}

/// Validates declaring peace: the pair must currently be at war.
/// Emits no event.
pub fn can_peace(world: &World, fa: &str, fb: &str) -> Result<(), String> {
    can_modify_standing(fa, fb)?;
    if get_relation(world, fa, fb) != RelationState::War {
        return Err(format!("'{fa}' and '{fb}' are not at war"));
    }
    Ok(())
}

/// Declares peace between two warring factions: resets the pair to
/// `(Neutral, 0)` and emits exactly one `peace_declared { a, b, tick }` event.
pub fn declare_peace(world: &mut World, fa: &str, fb: &str) -> Result<(), String> {
    can_peace(world, fa, fb)?;
    let (a, b) = DiplomacyState::canonical(fa, fb);
    let entry = world.diplomacy.get_or_create(&a, &b);
    entry.state = RelationState::Neutral;
    entry.standing = 0;
    let tick = u64::from(world.turn);
    world.send_event(
        "peace_declared",
        json!({
            "a": a,
            "b": b,
            "tick": tick,
        }),
    )?;
    Ok(())
}
