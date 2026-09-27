//! World-level diplomacy store: faction-pair relationships.
//!
//! Holds bilateral state between faction ids (not entities) keyed by the
//! canonical `(min, max)` pair, following the free-function accessor split of
//! `faction.rs` / `tech_tree.rs`: module-level state on [`World`] plus thin
//! accessors, with no per-entity owner required.
//!
//! Relationship state is explicit-only: it changes solely through war, peace,
//! and treaty transitions, never derived from entity `Reputation`
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

    /// Read-only relation lookup shared by the `World` getter and other hosts.
    pub fn query_relation(&self, fa: &str, fb: &str) -> RelationState {
        self.get(fa, fb).state
    }

    /// Read-only standing lookup shared by the `World` getter and other hosts.
    pub fn query_standing(&self, fa: &str, fb: &str) -> i64 {
        self.get(fa, fb).standing
    }

    /// Read-only treaty listing shared by the `World` getter and other hosts.
    pub fn query_treaties(&self, faction: Option<&str>) -> Vec<Treaty> {
        let mut out: Vec<Treaty> = self
            .treaties
            .values()
            .filter(|t| faction.is_none_or(|f| t.a == f || t.b == f))
            .cloned()
            .collect();
        out.sort_by_key(|t| t.id);
        out
    }

    /// Shared proposal validation behind `can_propose`. Emits no event.
    fn check_propose(&self, proposer: &str, other: &str, kind: &TreatyKind) -> Result<(), String> {
        can_modify_standing(proposer, other)?;
        if self.query_relation(proposer, other) == RelationState::War && *kind != TreatyKind::Peace
        {
            return Err(format!(
                "cannot propose {kind:?} while '{proposer}' and '{other}' are at war"
            ));
        }
        let (a, b) = Self::canonical(proposer, other);
        let duplicate = self.treaties.values().any(|t| {
            let (ta, tb) = Self::canonical(&t.a, &t.b);
            ta == a && tb == b && t.kind == *kind && is_live(t.status)
        });
        if duplicate {
            return Err(format!(
                "duplicate live {kind:?} treaty between '{proposer}' and '{other}'"
            ));
        }
        Ok(())
    }

    /// Shared acceptance validation behind `can_accept`. Emits no event.
    fn check_accept(&self, treaty_id: u64) -> Result<(), String> {
        match self.treaties.get(&treaty_id) {
            Some(t) if t.status == TreatyStatus::Proposed => Ok(()),
            Some(_) => Err(format!("treaty {treaty_id} is not proposed")),
            None => Err(format!("unknown treaty {treaty_id}")),
        }
    }

    /// Shared break validation behind `can_break`. Emits no event.
    fn check_break(&self, treaty_id: u64) -> Result<(), String> {
        match self.treaties.get(&treaty_id) {
            Some(t) if is_live(t.status) => Ok(()),
            Some(_) => Err(format!("treaty {treaty_id} is already settled")),
            None => Err(format!("unknown treaty {treaty_id}")),
        }
    }

    /// Shared war validation behind `can_war`. Emits no event.
    fn check_war(&self, fa: &str, fb: &str) -> Result<(), String> {
        can_modify_standing(fa, fb)?;
        if self.query_relation(fa, fb) == RelationState::War {
            return Err(format!("'{fa}' and '{fb}' are already at war"));
        }
        Ok(())
    }

    /// Shared peace validation behind `can_peace`. Emits no event.
    fn check_peace(&self, fa: &str, fb: &str) -> Result<(), String> {
        can_modify_standing(fa, fb)?;
        if self.query_relation(fa, fb) != RelationState::War {
            return Err(format!("'{fa}' and '{fb}' are not at war"));
        }
        Ok(())
    }

    /// Applies a treaty kind's state floor to its pair on acceptance.
    ///
    /// Floors never demote: `Allied` survives `Peace`/`NonAggression`, and
    /// `War` is left intact by `NonAggression`/`Alliance` (only peace
    /// transitions clear it). Accepting a `Peace` treaty on a war pair ends
    /// the war with the same end state as `declare_peace`: `(Neutral, 0)`.
    fn floor_on_accept(&mut self, kind: TreatyKind, fa: &str, fb: &str) {
        let (a, b) = Self::canonical(fa, fb);
        let entry = self.get_or_create(&a, &b);
        match kind {
            TreatyKind::Peace => {
                if entry.state == RelationState::War {
                    entry.state = RelationState::Neutral;
                    entry.standing = 0;
                } else if entry.state == RelationState::Hostile {
                    entry.state = RelationState::Neutral;
                }
            }
            TreatyKind::NonAggression => {
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

    /// Applies a symmetric standing delta to a faction pair (bounds enforced
    /// here, once) and returns the `relation_changed` event to emit.
    pub fn apply_modify_standing(
        &mut self,
        fa: &str,
        fb: &str,
        delta: i64,
    ) -> Result<PendingEvents, String> {
        can_modify_standing(fa, fb)?;
        let (a, b) = Self::canonical(fa, fb);
        let entry = self.get_or_create(&a, &b);
        let old_standing = entry.standing;
        let new_standing = old_standing
            .saturating_add(delta)
            .clamp(MIN_STANDING, MAX_STANDING);
        entry.standing = new_standing;
        let state = entry.state;
        Ok(vec![(
            "relation_changed".to_string(),
            json!({
                "a": a,
                "b": b,
                "old_standing": old_standing,
                "new_standing": new_standing,
                "state": state,
            }),
        )])
    }

    /// Records a proposed treaty and returns its id plus the
    /// `treaty_proposed` event to emit. `duration_ticks: None` never expires.
    pub fn apply_propose_treaty(
        &mut self,
        proposer: &str,
        other: &str,
        kind: TreatyKind,
        turn: u64,
        duration_ticks: Option<u64>,
    ) -> Result<(u64, PendingEvents), String> {
        self.check_propose(proposer, other, &kind)?;
        let id = self.next_treaty_id;
        self.next_treaty_id += 1;
        self.treaties.insert(
            id,
            Treaty {
                id,
                a: proposer.to_string(),
                b: other.to_string(),
                kind,
                status: TreatyStatus::Proposed,
                proposed_tick: turn,
                active_tick: None,
                expires_tick: duration_ticks.map(|d| turn.saturating_add(d)),
            },
        );
        Ok((
            id,
            vec![(
                "treaty_proposed".to_string(),
                json!({
                    "treaty_id": id,
                    "proposer": proposer,
                    "other": other,
                    "kind": kind,
                }),
            )],
        ))
    }

    /// Accepts a proposed treaty, applies the kind's state floor, and returns
    /// the `treaty_signed` event to emit.
    pub fn apply_accept_treaty(
        &mut self,
        treaty_id: u64,
        turn: u64,
    ) -> Result<PendingEvents, String> {
        self.check_accept(treaty_id)?;
        let treaty = self
            .treaties
            .get_mut(&treaty_id)
            .expect("treaty validated by check_accept");
        treaty.status = TreatyStatus::Active;
        treaty.active_tick = Some(turn);
        let (kind, a, b) = (treaty.kind, treaty.a.clone(), treaty.b.clone());
        self.floor_on_accept(kind, &a, &b);
        Ok(vec![(
            "treaty_signed".to_string(),
            json!({
                "treaty_id": treaty_id,
                "kind": kind,
            }),
        )])
    }

    /// Breaks a proposed or active treaty, applies the standing penalty
    /// (bounds enforced here, once), and returns the `treaty_broken` event.
    ///
    /// The penalty adjusts pair standing directly without a
    /// `relation_changed` event so each transition emits exactly one event.
    pub fn apply_break_treaty(&mut self, treaty_id: u64) -> Result<PendingEvents, String> {
        self.check_break(treaty_id)?;
        let treaty = self
            .treaties
            .get_mut(&treaty_id)
            .expect("treaty validated by check_break");
        treaty.status = TreatyStatus::Broken;
        let (a, b) = Self::canonical(&treaty.a.clone(), &treaty.b.clone());
        let entry = self.get_or_create(&a, &b);
        entry.standing = (entry.standing + TREATY_BREAK_PENALTY).clamp(MIN_STANDING, MAX_STANDING);
        Ok(vec![(
            "treaty_broken".to_string(),
            json!({
                "treaty_id": treaty_id,
                "penalty": TREATY_BREAK_PENALTY,
            }),
        )])
    }

    /// Expires every `Active` treaty due at `turn` and returns one
    /// `treaty_expired` event per treaty.
    ///
    /// Boundary-exact: `expires_tick <= turn` expires, `expires_tick > turn`
    /// survives, and `expires_tick: None` never expires. Deterministic
    /// collect-then-apply; no wall-clock or RNG.
    pub fn apply_expire_due(&mut self, turn: u64) -> Result<PendingEvents, String> {
        let due: Vec<u64> = self
            .treaties
            .values()
            .filter(|t| {
                t.status == TreatyStatus::Active && t.expires_tick.is_some_and(|e| e <= turn)
            })
            .map(|t| t.id)
            .collect();
        let mut pending = Vec::with_capacity(due.len());
        for id in due {
            if let Some(treaty) = self.treaties.get_mut(&id) {
                treaty.status = TreatyStatus::Expired;
            }
            pending.push(("treaty_expired".to_string(), json!({ "treaty_id": id })));
        }
        Ok(pending)
    }

    /// Declares war between two factions and returns the `war_declared` event
    /// followed by one `treaty_broken` event per auto-broken paper treaty.
    ///
    /// War breaks paper: every `Active` `NonAggression`/`Alliance`/`Peace`
    /// treaty on the pair moves to `Broken`. `TradeStub` treaties stay in
    /// force.
    pub fn apply_declare_war(
        &mut self,
        aggressor: &str,
        defender: &str,
        turn: u64,
    ) -> Result<PendingEvents, String> {
        self.check_war(aggressor, defender)?;
        let (a, b) = Self::canonical(aggressor, defender);
        self.get_or_create(&a, &b).state = RelationState::War;
        let mut pending = vec![(
            "war_declared".to_string(),
            json!({
                "aggressor": aggressor,
                "defender": defender,
                "tick": turn,
            }),
        )];
        let paper: Vec<u64> = self
            .treaties
            .values()
            .filter(|t| {
                let (ta, tb) = Self::canonical(&t.a, &t.b);
                ta == a
                    && tb == b
                    && t.status == TreatyStatus::Active
                    && matches!(
                        t.kind,
                        TreatyKind::NonAggression | TreatyKind::Alliance | TreatyKind::Peace
                    )
            })
            .map(|t| t.id)
            .collect();
        for id in paper {
            pending.extend(self.apply_break_treaty(id)?);
        }
        Ok(pending)
    }

    /// Declares peace between two warring factions (pair resets to
    /// `(Neutral, 0)`) and returns the `peace_declared` event to emit.
    pub fn apply_declare_peace(
        &mut self,
        fa: &str,
        fb: &str,
        turn: u64,
    ) -> Result<PendingEvents, String> {
        self.check_peace(fa, fb)?;
        let (a, b) = Self::canonical(fa, fb);
        let entry = self.get_or_create(&a, &b);
        entry.state = RelationState::Neutral;
        entry.standing = 0;
        Ok(vec![(
            "peace_declared".to_string(),
            json!({
                "a": a,
                "b": b,
                "tick": turn,
            }),
        )])
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

/// Pending events produced by one diplomacy transition, in emission order.
///
/// The `World` wrappers forward each entry to the ECS event bus; other hosts
/// (notably WASM, whose world type cannot call the `World` free functions)
/// forward them to their own bus. Building payloads here keeps a single
/// implementation of every transition rule and payload shape.
pub type PendingEvents = Vec<(String, serde_json::Value)>;

impl RelationState {
    /// Canonical lowercase relation name shared by every scripting bridge.
    pub fn as_str(self) -> &'static str {
        match self {
            RelationState::Allied => "allied",
            RelationState::Neutral => "neutral",
            RelationState::Hostile => "hostile",
            RelationState::War => "war",
        }
    }
}

impl TreatyKind {
    /// Canonical lowercase kind name shared by every scripting bridge.
    pub fn as_str(self) -> &'static str {
        match self {
            TreatyKind::NonAggression => "non_aggression",
            TreatyKind::Alliance => "alliance",
            TreatyKind::Peace => "peace",
            TreatyKind::TradeStub => "trade",
        }
    }

    /// Parses a bridge kind string; rejects anything outside the closed set.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "non_aggression" => Ok(TreatyKind::NonAggression),
            "alliance" => Ok(TreatyKind::Alliance),
            "peace" => Ok(TreatyKind::Peace),
            "trade" => Ok(TreatyKind::TradeStub),
            _ => Err(format!("unknown treaty kind '{s}'")),
        }
    }
}

impl TreatyStatus {
    /// Canonical lowercase status name shared by every scripting bridge.
    pub fn as_str(self) -> &'static str {
        match self {
            TreatyStatus::Proposed => "proposed",
            TreatyStatus::Active => "active",
            TreatyStatus::Broken => "broken",
            TreatyStatus::Expired => "expired",
        }
    }
}

impl Treaty {
    /// Canonical JSON record shared by every scripting bridge, so all
    /// languages observe identical treaty shapes.
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "a": self.a,
            "b": self.b,
            "kind": self.kind.as_str(),
            "status": self.status.as_str(),
            "proposed_tick": self.proposed_tick,
            "active_tick": self.active_tick,
            "expires_tick": self.expires_tick,
        })
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
    world.diplomacy.query_relation(fa, fb)
}

/// Returns the standing score for a faction pair.
///
/// Unknown pairs (and self-pairs) report `0`. Lookup is order-independent.
pub fn get_standing(world: &World, fa: &str, fb: &str) -> i64 {
    world.diplomacy.query_standing(fa, fb)
}

/// Forwards core-produced transition events to the ECS event bus.
fn forward_pending(world: &mut World, pending: PendingEvents) -> Result<(), String> {
    for (name, payload) in pending {
        world.send_event(&name, payload)?;
    }
    Ok(())
}

/// Applies a symmetric standing delta to a faction pair, clamped to
/// `MIN_STANDING..=MAX_STANDING`, and emits exactly one `relation_changed`
/// event carrying `{ a, b, old_standing, new_standing, state }` in canonical
/// pair order.
///
/// Relationship state is preserved: standing never promotes or demotes state
/// implicitly. Self-pair and empty-id calls return `Err` and emit no event.
///
/// Thin wrapper over [`DiplomacyState::apply_modify_standing`].
pub fn modify_standing(world: &mut World, fa: &str, fb: &str, delta: i64) -> Result<(), String> {
    let pending = world.diplomacy.apply_modify_standing(fa, fb, delta)?;
    forward_pending(world, pending)
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
    world.diplomacy.check_propose(proposer, other, kind)
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
///
/// Thin wrapper over [`DiplomacyState::apply_propose_treaty`].
pub fn propose_treaty(
    world: &mut World,
    proposer: &str,
    other: &str,
    kind: TreatyKind,
    duration_ticks: Option<u64>,
) -> Result<u64, String> {
    let turn = u64::from(world.turn);
    let (id, pending) =
        world
            .diplomacy
            .apply_propose_treaty(proposer, other, kind, turn, duration_ticks)?;
    forward_pending(world, pending)?;
    Ok(id)
}

/// Validates a treaty acceptance: the record must exist and be `Proposed`.
/// Emits no event.
pub fn can_accept(world: &World, treaty_id: u64) -> Result<(), String> {
    world.diplomacy.check_accept(treaty_id)
}

/// Accepts a proposed treaty: sets `Active` with `active_tick = turn`, applies
/// the kind's state floor (`Peace`/`NonAggression` lift `Hostile` to `Neutral`,
/// `Alliance` raises to `Allied` unless at war, `TradeStub` changes nothing),
/// and emits exactly one `treaty_signed { treaty_id, kind }` event.
///
/// Thin wrapper over [`DiplomacyState::apply_accept_treaty`].
pub fn accept_treaty(world: &mut World, treaty_id: u64) -> Result<(), String> {
    let turn = u64::from(world.turn);
    let pending = world.diplomacy.apply_accept_treaty(treaty_id, turn)?;
    forward_pending(world, pending)
}

/// Validates breaking a treaty: the record must exist and be
/// `Proposed` or `Active`. Emits no event.
pub fn can_break(world: &World, treaty_id: u64) -> Result<(), String> {
    world.diplomacy.check_break(treaty_id)
}

/// Breaks a proposed or active treaty: sets `Broken`, applies the
/// `TREATY_BREAK_PENALTY` standing penalty (clamped), and emits exactly one
/// `treaty_broken { treaty_id, penalty }` event.
///
/// The penalty adjusts pair standing directly without a `relation_changed`
/// event so each transition emits exactly one event.
///
/// Thin wrapper over [`DiplomacyState::apply_break_treaty`].
pub fn break_treaty(world: &mut World, treaty_id: u64) -> Result<(), String> {
    let pending = world.diplomacy.apply_break_treaty(treaty_id)?;
    forward_pending(world, pending)
}

/// Expires every `Active` treaty whose `expires_tick` has reached the current
/// turn, emitting one `treaty_expired { treaty_id }` event per treaty.
///
/// Boundary-exact: `expires_tick <= turn` expires, `expires_tick > turn`
/// survives, and `expires_tick: None` never expires. Deterministic
/// collect-then-apply; no wall-clock or RNG.
///
/// Thin wrapper over [`DiplomacyState::apply_expire_due`].
pub fn expire_due_treaties(world: &mut World) -> Result<(), String> {
    let turn = u64::from(world.turn);
    let pending = world.diplomacy.apply_expire_due(turn)?;
    forward_pending(world, pending)
}

/// Lists treaty records, optionally filtered to one faction's membership.
pub fn list_treaties(world: &World, faction: Option<&str>) -> Vec<Treaty> {
    world.diplomacy.query_treaties(faction)
}

/// Validates a war declaration: distinct non-empty ids, pair not already at war.
/// Emits no event.
pub fn can_war(world: &World, fa: &str, fb: &str) -> Result<(), String> {
    world.diplomacy.check_war(fa, fb)
}

/// Declares war between two factions: sets pair state to `War` and emits
/// exactly one `war_declared { aggressor, defender, tick }` event.
///
/// War breaks paper: every `Active` `NonAggression`/`Alliance`/`Peace` treaty
/// on the pair moves to `Broken` with one `treaty_broken { treaty_id, penalty }`
/// event each (same penalty path as [`break_treaty`]). `TradeStub` treaties
/// stay in force.
///
/// Thin wrapper over [`DiplomacyState::apply_declare_war`].
pub fn declare_war(world: &mut World, aggressor: &str, defender: &str) -> Result<(), String> {
    let turn = u64::from(world.turn);
    let pending = world
        .diplomacy
        .apply_declare_war(aggressor, defender, turn)?;
    forward_pending(world, pending)
}

/// Validates declaring peace: the pair must currently be at war.
/// Emits no event.
pub fn can_peace(world: &World, fa: &str, fb: &str) -> Result<(), String> {
    world.diplomacy.check_peace(fa, fb)
}

/// Declares peace between two warring factions: resets the pair to
/// `(Neutral, 0)` and emits exactly one `peace_declared { a, b, tick }` event.
///
/// Thin wrapper over [`DiplomacyState::apply_declare_peace`].
pub fn declare_peace(world: &mut World, fa: &str, fb: &str) -> Result<(), String> {
    let turn = u64::from(world.turn);
    let pending = world.diplomacy.apply_declare_peace(fa, fb, turn)?;
    forward_pending(world, pending)
}
