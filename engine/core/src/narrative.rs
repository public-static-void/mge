//! Event-driven narrative engine: scenario definitions and incident-director state.
//!
//! Holds the pool of registered scenario definitions plus the world-scoped
//! narrative store on [`World`](crate::ecs::world::World), following the
//! free-function accessor split of `diplomacy.rs` / `tech_tree.rs`:
//! module-level state on [`World`] plus thin accessors, with no per-entity
//! owner required.
//!
//! This module covers registration, state shape, per-tick trigger evaluation
//! with weighted RNG selection, and the pending-decision lifecycle (resolve,
//! expiry, cooldown) with choice-effect dispatch. History records and
//! save/load round-trips build on the same state in later milestones.

use crate::diplomacy::{PendingEvents, get_relation, get_standing};
use crate::ecs::world::World;
use crate::tech_tree::Effect;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Narrative errors are human-readable rejections; failed calls change no state.
pub type NarrativeError = String;

/// Default selection weight for scenarios that omit it.
fn default_weight() -> f64 {
    1.0
}

/// A single trigger predicate. Predicates combine with AND semantics; an
/// [`TriggerPredicate::Unknown`] entry renders its scenario ineligible for
/// firing without failing registration or the tick.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TriggerPredicate {
    /// Fires once the world turn reaches `turn`.
    #[serde(rename = "turn_gte")]
    TurnGte {
        /// Minimum turn (inclusive).
        turn: u64,
    },
    /// Fires while at least `count` entities carry `component`.
    #[serde(rename = "entity_count_gte")]
    EntityCountGte {
        /// Component name to count.
        component: String,
        /// Minimum entity count (inclusive).
        count: usize,
    },
    /// Fires while the standing of pair `(a, b)` is at or below `value`.
    #[serde(rename = "faction_standing_lte")]
    FactionStandingLte {
        /// First faction id.
        a: String,
        /// Second faction id.
        b: String,
        /// Upper bound (inclusive).
        value: i64,
    },
    /// Fires while the standing of pair `(a, b)` is at or above `value`.
    #[serde(rename = "faction_standing_gte")]
    FactionStandingGte {
        /// First faction id.
        a: String,
        /// Second faction id.
        b: String,
        /// Lower bound (inclusive).
        value: i64,
    },
    /// Fires while the relation of pair `(a, b)` equals `relation`.
    #[serde(rename = "faction_relation_is")]
    FactionRelationIs {
        /// First faction id.
        a: String,
        /// Second faction id.
        b: String,
        /// Canonical relation string (`allied`/`neutral`/`hostile`/`war`).
        relation: String,
    },
    /// Fires once `scenario_id` has a prior resolution on record.
    #[serde(rename = "scenario_resolved")]
    ScenarioResolved {
        /// Scenario id that must have resolved before.
        scenario_id: String,
    },
    /// Probabilistic gate evaluated on the narrative seeded RNG stream.
    #[serde(rename = "chance")]
    Chance {
        /// Fire probability in `[0, 1]` (clamped at evaluation).
        p: f64,
    },
    /// Any predicate type outside the known language. Never an error;
    /// the owning scenario simply never fires on this entry.
    #[serde(other)]
    Unknown,
}

/// One resolvable choice on a scenario, carrying ordered effects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenarioChoice {
    /// Choice id, unique within its scenario.
    pub id: String,
    /// Player-facing label.
    pub label: String,
    /// Effects applied in order on resolution.
    #[serde(default)]
    pub effects: Vec<Effect>,
}

/// A registered scenario definition: trigger predicates plus decision choices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenarioDef {
    /// Unique non-empty scenario id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description text.
    #[serde(default)]
    pub description: String,
    /// Trigger predicates, combined with AND semantics.
    #[serde(default)]
    pub triggers: Vec<TriggerPredicate>,
    /// Resolvable choices (at least one required).
    pub choices: Vec<ScenarioChoice>,
    /// Refire blackout in turns after resolution or expiry.
    #[serde(default)]
    pub cooldown_turns: u64,
    /// Pending-decision timeout in turns from firing.
    #[serde(default)]
    pub expires_in_turns: Option<u64>,
    /// Resolved scenarios with `once` set never refire.
    #[serde(default)]
    pub once: bool,
    /// Relative selection weight among simultaneously-eligible scenarios.
    #[serde(default = "default_weight")]
    pub weight: f64,
}

/// A fired scenario awaiting a resolve/expire transition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingDecision {
    /// Monotonic decision id, never reused.
    pub id: u64,
    /// Scenario that fired.
    pub scenario_id: String,
    /// Turn the decision fired on.
    pub fired_tick: u64,
    /// Turn the decision expires on, if it times out.
    #[serde(default)]
    pub expires_tick: Option<u64>,
    /// Choices copied from the scenario definition at fire time.
    pub choices: Vec<ScenarioChoice>,
}

/// Lifecycle transition recorded in narrative history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeRecordKind {
    /// A decision fired.
    Fired,
    /// A decision resolved with a choice.
    Resolved,
    /// A pending decision timed out.
    Expired,
}

/// Turn-indexed history entry for fire/resolve/expire transitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NarrativeRecord {
    /// Scenario the record belongs to.
    pub scenario_id: String,
    /// Decision the record belongs to.
    pub decision_id: u64,
    /// Transition kind.
    pub kind: NarrativeRecordKind,
    /// World turn of the transition (never wall-clock).
    pub turn: u64,
    /// Resolving choice id, set on resolved records only.
    #[serde(default)]
    pub choice_id: Option<String>,
}

/// World-level narrative store, held on [`World`] and serialized for
/// whole-world save/load. Every field carries `#[serde(default)]` (directly
/// or via the container default) so pre-narrative saves load empty.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NarrativeState {
    /// Registered scenario definitions by id.
    #[serde(default)]
    pub scenarios: HashMap<String, ScenarioDef>,
    /// Live pending decisions by monotonic id.
    #[serde(default)]
    pub pending: HashMap<u64, PendingDecision>,
    /// Append-only turn-indexed transition history.
    #[serde(default)]
    pub history: Vec<NarrativeRecord>,
    /// Scenario id → earliest turn it may refire.
    #[serde(default)]
    pub cooldowns: HashMap<String, u64>,
    /// Next pending-decision id counter. Defaults zero for old saves.
    #[serde(default)]
    pub next_decision_id: u64,
    /// Deterministic RNG seed, persisted across ticks for save/load determinism.
    #[serde(default)]
    pub rng_state: [u8; 32],
}

impl NarrativeState {
    /// Validates and stores a scenario definition.
    ///
    /// Registering a duplicate `id` replaces the definition; tick evaluation
    /// keys on the map so replacement never duplicates firing. Rejections
    /// (empty id, zero choices, empty choice id) leave prior state untouched.
    pub fn register_scenario(&mut self, def: ScenarioDef) -> Result<(), NarrativeError> {
        validate_scenario(&def)?;
        self.scenarios.insert(def.id.clone(), def);
        Ok(())
    }

    /// Evaluates one narrative tick against a prebuilt snapshot.
    ///
    /// Collect-then-apply: timed-out pending decisions expire first (expired
    /// record, `narrative_expired` event, cooldown write, no effects), then
    /// every registered scenario is scanned in `id` order (`chance` draws
    /// consume the seeded stream in that order), at most one eligible
    /// scenario fires per tick by `weight`-proportional selection, and the
    /// win is applied (pending entry, fired history record, fired event)
    /// before returning. A scenario stays ineligible while it holds a live
    /// pending decision, waits out a cooldown, resolved with `once` set,
    /// expired on this same tick, carries a non-positive or non-finite
    /// weight, or has any unmet (or unknown) predicate. The RNG stream
    /// advances every tick so seeded replays stay aligned. Never fails: an
    /// empty pool returns empty vecs.
    pub fn apply_tick(
        &mut self,
        snap: &NarrativeSnapshot,
    ) -> (Vec<PendingDecision>, PendingEvents) {
        let mut rng = SmallRng::from_seed(self.rng_state);
        let mut events: PendingEvents = Vec::new();
        let mut just_expired: HashSet<String> = HashSet::new();
        let timed_out: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, decision)| decision.expires_tick.is_some_and(|end| end <= snap.turn))
            .map(|(id, _)| *id)
            .collect();
        for id in timed_out {
            if let Some(decision) = self.pending.remove(&id) {
                let blackout = self
                    .scenarios
                    .get(&decision.scenario_id)
                    .map(|def| def.cooldown_turns)
                    .unwrap_or(0);
                self.cooldowns.insert(
                    decision.scenario_id.clone(),
                    snap.turn.saturating_add(blackout),
                );
                self.history.push(NarrativeRecord {
                    scenario_id: decision.scenario_id.clone(),
                    decision_id: id,
                    kind: NarrativeRecordKind::Expired,
                    turn: snap.turn,
                    choice_id: None,
                });
                just_expired.insert(decision.scenario_id.clone());
                events.push((
                    "narrative_expired".to_string(),
                    serde_json::json!({
                        "decision_id": id,
                        "scenario_id": decision.scenario_id,
                        "expired_tick": snap.turn,
                    }),
                ));
            }
        }
        let mut ordered: Vec<&ScenarioDef> = self.scenarios.values().collect();
        ordered.sort_by(|a, b| a.id.cmp(&b.id));
        let mut eligible: Vec<&ScenarioDef> = Vec::new();
        for def in ordered {
            if self.is_pending(&def.id) {
                continue;
            }
            if just_expired.contains(&def.id) {
                continue;
            }
            if self.on_cooldown(&def.id, snap.turn) {
                continue;
            }
            if def.once && snap.resolved.contains(&def.id) {
                continue;
            }
            if !def.weight.is_finite() || def.weight <= 0.0 {
                continue;
            }
            if predicates_hold(&def.triggers, snap, &mut rng) {
                eligible.push(def);
            }
        }
        let fired = pick_weighted(&eligible, &mut rng).cloned();
        rng.fill(&mut self.rng_state);
        match fired {
            None => (Vec::new(), events),
            Some(def) => {
                let id = self.next_decision_id;
                self.next_decision_id += 1;
                let decision = PendingDecision {
                    id,
                    scenario_id: def.id.clone(),
                    fired_tick: snap.turn,
                    expires_tick: def.expires_in_turns.map(|span| snap.turn + span),
                    choices: def.choices.clone(),
                };
                self.pending.insert(id, decision.clone());
                self.history.push(NarrativeRecord {
                    scenario_id: def.id.clone(),
                    decision_id: id,
                    kind: NarrativeRecordKind::Fired,
                    turn: snap.turn,
                    choice_id: None,
                });
                events.push((
                    "narrative_fired".to_string(),
                    serde_json::json!({
                        "decision_id": id,
                        "scenario_id": def.id,
                        "fired_tick": snap.turn,
                    }),
                ));
                (vec![decision], events)
            }
        }
    }

    /// Resolves a pending decision with one of its choices.
    ///
    /// Removes the pending entry, writes the post-resolution cooldown
    /// (`turn + cooldown_turns`), appends a resolved history record, and
    /// returns the choice's effects with the `narrative_resolved` event for
    /// the caller to dispatch and forward. Resolving an unknown or already
    /// settled decision, or an unknown choice, errors with no state change.
    pub fn apply_resolve(
        &mut self,
        decision_id: u64,
        choice_id: &str,
        turn: u64,
    ) -> Result<(Vec<Effect>, PendingEvents), NarrativeError> {
        let decision = self
            .pending
            .get(&decision_id)
            .cloned()
            .ok_or_else(|| format!("Unknown or settled narrative decision {decision_id}"))?;
        let choice = decision
            .choices
            .iter()
            .find(|candidate| candidate.id == choice_id)
            .ok_or_else(|| {
                format!(
                    "Unknown choice '{choice_id}' for scenario '{}'",
                    decision.scenario_id
                )
            })?;
        let effects = choice.effects.clone();
        let scenario_id = decision.scenario_id.clone();
        self.pending.remove(&decision_id);
        let blackout = self
            .scenarios
            .get(&scenario_id)
            .map(|def| def.cooldown_turns)
            .unwrap_or(0);
        self.cooldowns
            .insert(scenario_id.clone(), turn.saturating_add(blackout));
        self.history.push(NarrativeRecord {
            scenario_id: scenario_id.clone(),
            decision_id,
            kind: NarrativeRecordKind::Resolved,
            turn,
            choice_id: Some(choice_id.to_string()),
        });
        let events = vec![(
            "narrative_resolved".to_string(),
            serde_json::json!({
                "decision_id": decision_id,
                "scenario_id": scenario_id,
                "choice_id": choice_id,
                "resolved_tick": turn,
            }),
        )];
        Ok((effects, events))
    }

    /// Returns true while a live pending decision exists for the scenario.
    fn is_pending(&self, scenario_id: &str) -> bool {
        self.pending
            .values()
            .any(|decision| decision.scenario_id == scenario_id)
    }

    /// Returns true while the scenario waits out a post-resolution cooldown.
    fn on_cooldown(&self, scenario_id: &str, turn: u64) -> bool {
        self.cooldowns
            .get(scenario_id)
            .is_some_and(|ready| turn < *ready)
    }
}

/// Owned read-only view of the world state narrative triggers observe.
///
/// Built from `&World` ahead of the tick so evaluation never holds a world
/// borrow while `NarrativeState` mutates (collect-then-apply), and so other
/// hosts (notably WASM) can drive the same pure [`NarrativeState::apply_tick`]
/// rules from their own world type.
#[derive(Debug, Clone, Default)]
pub struct NarrativeSnapshot {
    /// World turn at tick time.
    pub turn: u64,
    /// Entities carrying each queried component, by component name.
    pub component_counts: HashMap<String, usize>,
    /// Standing per canonical `(min, max)` faction pair.
    pub standings: HashMap<(String, String), i64>,
    /// Canonical lowercase relation per canonical `(min, max)` pair.
    pub relations: HashMap<(String, String), String>,
    /// Scenario ids with at least one resolved history record.
    pub resolved: HashSet<String>,
}

/// Canonical `(min, max)` key for a faction pair (diplomacy precedent).
fn canonical_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

/// Builds the snapshot for one tick: collects exactly the component counts,
/// faction pairs, and resolution flags the registered triggers query, so the
/// tick scan costs `O(scenarios x predicates)` with no per-entity iteration
/// beyond the requested count queries.
pub fn snapshot_narrative_view(world: &World) -> NarrativeSnapshot {
    let mut components: HashSet<String> = HashSet::new();
    let mut pairs: HashSet<(String, String)> = HashSet::new();
    for def in world.narrative.scenarios.values() {
        for trigger in &def.triggers {
            match trigger {
                TriggerPredicate::EntityCountGte { component, .. } => {
                    components.insert(component.clone());
                }
                TriggerPredicate::FactionStandingLte { a, b, .. }
                | TriggerPredicate::FactionStandingGte { a, b, .. }
                | TriggerPredicate::FactionRelationIs { a, b, .. } => {
                    pairs.insert(canonical_pair(a, b));
                }
                _ => {}
            }
        }
    }
    let mut snap = NarrativeSnapshot {
        turn: u64::from(world.turn),
        ..Default::default()
    };
    for component in components {
        snap.component_counts.insert(
            component.clone(),
            world.get_entities_with_component(&component).len(),
        );
    }
    for (a, b) in pairs {
        snap.standings
            .insert((a.clone(), b.clone()), get_standing(world, &a, &b));
        snap.relations.insert(
            (a.clone(), b.clone()),
            get_relation(world, &a, &b).as_str().to_string(),
        );
    }
    for record in &world.narrative.history {
        if record.kind == NarrativeRecordKind::Resolved {
            snap.resolved.insert(record.scenario_id.clone());
        }
    }
    snap
}

/// Runs one narrative tick on a live world: snapshot, apply, forward events.
///
/// Thin wrapper over [`snapshot_narrative_view`] plus
/// [`NarrativeState::apply_tick`] so the system entry point and tests share
/// one path. Event forwarding never runs under a state borrow.
pub fn tick_narrative(world: &mut World) {
    let snapshot = snapshot_narrative_view(world);
    let (_decisions, pending) = world.narrative.apply_tick(&snapshot);
    for (name, payload) in pending {
        let _ = world.send_event(&name, payload);
    }
}

/// Dispatches narrative choice effects in order.
///
/// Two built-in narrative-local actions run here: `emit_event`
/// (`{ bus, payload }` forwards to [`World::send_event`]) and
/// `modify_standing` (`{ a, b, delta }` adjusts the diplomacy standing).
/// Any other action passes through the already-registered
/// `EffectProcessorRegistry` handlers (collected under the lock, invoked
/// outside it); the target entity id comes from `data.entity` when present
/// (world-scoped effects pass `0`). An effect naming a missing or despawned
/// entity, an unknown action, or a malformed payload is skipped without
/// aborting the remaining effects, so dispatch never fails the tick.
pub fn dispatch_narrative_effects(world: &mut World, effects: &[Effect]) {
    for effect in effects {
        let data = effect.data.clone().unwrap_or_default();
        let target = data
            .get("entity")
            .and_then(|v| v.as_u64())
            .map(|id| id as u32);
        if let Some(entity) = target
            && !world.entity_exists(entity)
        {
            continue;
        }
        match effect.action.as_str() {
            "emit_event" => {
                let Some(bus) = data.get("bus").and_then(|v| v.as_str()) else {
                    continue;
                };
                if bus.is_empty() {
                    continue;
                }
                let payload = data.get("payload").cloned().unwrap_or_default();
                let _ = world.send_event(bus, payload);
            }
            "modify_standing" => {
                let (Some(a), Some(b), Some(delta)) = (
                    data.get("a").and_then(|v| v.as_str()),
                    data.get("b").and_then(|v| v.as_str()),
                    data.get("delta").and_then(|v| v.as_i64()),
                ) else {
                    continue;
                };
                let _ = crate::diplomacy::modify_standing(world, a, b, delta);
            }
            action => {
                let handler = world
                    .effect_processor_registry
                    .as_ref()
                    .and_then(|registry| registry.lock().unwrap().handler_for(action));
                match handler {
                    Some(invokable) => {
                        let value = serde_json::to_value(effect).unwrap_or_default();
                        invokable(world, target.unwrap_or(0), &value);
                    }
                    None => {
                        log::warn!(
                            "Narrative effect action '{action}' has no registered handler; skipping"
                        );
                    }
                }
            }
        }
    }
}

/// Resolves a pending decision with one of its choices.
///
/// Applies the pure [`NarrativeState::apply_resolve`] transition, dispatches
/// the choice's effects in order, then forwards the `narrative_resolved`
/// event. Failed calls (unknown or settled decision, unknown choice) change
/// no state; effect dispatch itself never fails.
pub fn resolve_decision(
    world: &mut World,
    decision_id: u64,
    choice_id: &str,
) -> Result<(), NarrativeError> {
    let turn = u64::from(world.turn);
    let (effects, pending) = world
        .narrative
        .apply_resolve(decision_id, choice_id, turn)?;
    dispatch_narrative_effects(world, &effects);
    for (name, payload) in pending {
        let _ = world.send_event(&name, payload);
    }
    Ok(())
}

/// Evaluates a predicate list with AND semantics.
fn predicates_hold(
    triggers: &[TriggerPredicate],
    snap: &NarrativeSnapshot,
    rng: &mut SmallRng,
) -> bool {
    triggers
        .iter()
        .all(|trigger| predicate_holds(trigger, snap, rng))
}

/// Evaluates one predicate. `chance` draws from the shared seeded stream;
/// boundary probabilities short-circuit without drawing. Unknown predicates
/// are ineligible without failing.
fn predicate_holds(
    predicate: &TriggerPredicate,
    snap: &NarrativeSnapshot,
    rng: &mut SmallRng,
) -> bool {
    match predicate {
        TriggerPredicate::TurnGte { turn } => snap.turn >= *turn,
        TriggerPredicate::EntityCountGte { component, count } => {
            snap.component_counts.get(component).copied().unwrap_or(0) >= *count
        }
        TriggerPredicate::FactionStandingLte { a, b, value } => {
            snap.standings
                .get(&canonical_pair(a, b))
                .copied()
                .unwrap_or(0)
                <= *value
        }
        TriggerPredicate::FactionStandingGte { a, b, value } => {
            snap.standings
                .get(&canonical_pair(a, b))
                .copied()
                .unwrap_or(0)
                >= *value
        }
        TriggerPredicate::FactionRelationIs { a, b, relation } => {
            snap.relations
                .get(&canonical_pair(a, b))
                .map(String::as_str)
                .unwrap_or("neutral")
                == relation.to_ascii_lowercase()
        }
        TriggerPredicate::ScenarioResolved { scenario_id } => snap.resolved.contains(scenario_id),
        TriggerPredicate::Chance { p } => {
            if *p <= 0.0 {
                false
            } else if *p >= 1.0 {
                true
            } else {
                rng.random::<f64>() < *p
            }
        }
        TriggerPredicate::Unknown => false,
    }
}

/// Picks one eligible scenario proportionally to `weight`. All callers filter
/// non-positive weights first, so the total is positive; the trailing
/// fallback only guards float rounding.
fn pick_weighted<'a>(eligible: &[&'a ScenarioDef], rng: &mut SmallRng) -> Option<&'a ScenarioDef> {
    let total: f64 = eligible.iter().map(|def| def.weight).sum();
    if total <= 0.0 {
        return None;
    }
    let mut roll = rng.random_range(0.0..total);
    for def in eligible {
        roll -= def.weight;
        if roll < 0.0 {
            return Some(def);
        }
    }
    eligible.last().copied()
}

/// Shared definition validation behind [`register_scenario`].
fn validate_scenario(def: &ScenarioDef) -> Result<(), NarrativeError> {
    if def.id.trim().is_empty() {
        return Err("Scenario id must be non-empty".to_string());
    }
    if def.choices.is_empty() {
        return Err(format!(
            "Scenario '{}' must declare at least one choice",
            def.id
        ));
    }
    for choice in &def.choices {
        if choice.id.trim().is_empty() {
            return Err(format!(
                "Scenario '{}' has a choice with an empty id",
                def.id
            ));
        }
    }
    Ok(())
}

/// Registers a scenario definition from its JSON encoding.
///
/// Malformed JSON and invalid definitions are rejected with an error and
/// leave prior narrative state untouched.
pub fn register_scenario(world: &mut World, def_json: &str) -> Result<(), NarrativeError> {
    let def: ScenarioDef =
        serde_json::from_str(def_json).map_err(|e| format!("Invalid scenario definition: {e}"))?;
    world.narrative.register_scenario(def)
}

/// Lists all registered scenario definitions in id-sorted order.
pub fn list_scenarios(world: &World) -> Vec<ScenarioDef> {
    let mut out: Vec<ScenarioDef> = world.narrative.scenarios.values().cloned().collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Returns the registered scenario definition for an id, if present.
pub fn get_scenario(world: &World, id: &str) -> Option<ScenarioDef> {
    world.narrative.scenarios.get(id).cloned()
}

/// Lists live pending decisions in id order.
///
/// Bridges poll this after a tick to discover fired scenarios; id order
/// keeps every surface deterministic.
pub fn list_pending_decisions(world: &World) -> Vec<PendingDecision> {
    let mut out: Vec<PendingDecision> = world.narrative.pending.values().cloned().collect();
    out.sort_by_key(|decision| decision.id);
    out
}

/// Returns the live pending decision for an id, if still unresolved.
pub fn get_pending_decision(world: &World, id: u64) -> Option<PendingDecision> {
    world.narrative.pending.get(&id).cloned()
}

/// Returns the turn-indexed narrative history in append order.
///
/// Records use `world.turn`, never wall-clock timestamps, so follow-on
/// lore generation can consume them directly.
pub fn get_narrative_history(world: &World) -> Vec<NarrativeRecord> {
    world.narrative.history.clone()
}

/// Candidate content-file locations, mirroring the `tech_tree.rs` idiom:
/// workspace root, engine subdir, then the core-crate test depth.
fn scenario_file_paths() -> [&'static str; 3] {
    [
        "engine/assets/schemas/scenarios.json", // from workspace root
        "../engine/assets/schemas/scenarios.json", // from engine/ subdir
        "../../engine/assets/schemas/scenarios.json", // from engine/core/ subdir (tests)
    ]
}

/// Loads scenario definitions from the `scenarios.json` content file.
///
/// Returns an empty vec when the file is missing or unparsable; scenario
/// content is data, and a missing file must never fail world startup.
pub fn load_scenario_definitions() -> Vec<ScenarioDef> {
    for path_str in scenario_file_paths() {
        let path = Path::new(path_str);
        if !path.exists() {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(path)
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(items) = json.get("scenarios").and_then(|v| v.as_array())
        {
            return items
                .iter()
                .filter_map(|item| serde_json::from_value(item.clone()).ok())
                .collect();
        }
        break;
    }
    Vec::new()
}
