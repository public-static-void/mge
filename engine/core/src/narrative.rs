//! Event-driven narrative engine: scenario definitions and incident-director state.
//!
//! Holds the pool of registered scenario definitions plus the world-scoped
//! narrative store on [`World`](crate::ecs::world::World), following the
//! free-function accessor split of `diplomacy.rs` / `tech_tree.rs`:
//! module-level state on [`World`] plus thin accessors, with no per-entity
//! owner required.
//!
//! This module covers registration and state shape. Tick evaluation,
//! the pending-decision lifecycle, and history recording arrive in later
//! milestones; the state fields they need are reserved here so saves
//! written now stay load-compatible.

use crate::ecs::world::World;
use crate::tech_tree::Effect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
