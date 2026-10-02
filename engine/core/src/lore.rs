//! Procedural history and lore: world-level chronicle read-model.
//!
//! Holds the persistent [`LoreState`] store on [`World`](crate::ecs::world::World),
//! following the free-function accessor split of `narrative.rs`: module-level
//! state on [`World`] plus thin accessors, with no per-entity owner required.
//!
//! Covers the store shape, persistence wiring, the monotonic append path, the
//! live narrative feed, filtered queries, deterministic text rendering, and
//! the seeded founding backfill. Bridges (Lua/Python/WASM) arrive in later
//! milestones and build on this API.

use crate::ecs::world::World;
use crate::narrative::{NarrativeRecord, NarrativeRecordKind, ScenarioDef};
use rand::Rng;
use rand::RngCore;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lore errors are human-readable rejections; failed calls change no state.
pub type LoreError = String;

/// Lifecycle transition recorded in the chronicle, mirroring the narrative
/// record kinds plus a pre-play founding kind for backfilled eras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChronicleKind {
    /// A decision fired.
    Fired,
    /// A decision resolved with a choice.
    Resolved,
    /// A pending decision timed out.
    Expired,
    /// A pre-play founding-era entry from backfill generation.
    Founding,
}

/// One human-readable history entry in the world chronicle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChronicleEntry {
    /// Monotonic entry id, never reused.
    pub entry_id: u64,
    /// World turn of the entry (backfill entries predate live play).
    pub turn: u64,
    /// Owning scenario id, or `founding:{era}` for backfilled entries.
    pub scenario_id: String,
    /// Transition kind.
    pub kind: ChronicleKind,
    /// Pre-rendered summary via the fixed chronicle template.
    pub summary: String,
    /// Resolving choice id, set on resolved entries only.
    #[serde(default)]
    pub choice_id: Option<String>,
}

/// Conjunctive filter over chronicle entries; every `None` field is unbounded.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ChronicleFilter {
    /// Keep only entries for this scenario id.
    #[serde(default)]
    pub scenario_id: Option<String>,
    /// Keep only entries of this kind.
    #[serde(default)]
    pub kind: Option<ChronicleKind>,
    /// Keep only entries at or after this turn.
    #[serde(default)]
    pub turn_from: Option<u64>,
    /// Keep only entries at or before this turn.
    #[serde(default)]
    pub turn_to: Option<u64>,
}

/// World-level lore store, held on [`World`] and serialized for whole-world
/// save/load. Every field carries `#[serde(default)]` so pre-lore saves load
/// with an empty chronicle and a fresh RNG stream.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoreState {
    /// Append-only chronicle entries in insertion order.
    #[serde(default)]
    pub entries: Vec<ChronicleEntry>,
    /// Deterministic RNG stream for backfill generation, persisted across
    /// save/load following the narrative `rng_state` pattern.
    #[serde(default)]
    pub rng_state: [u8; 32],
    /// Next chronicle entry id counter. Defaults zero for old saves.
    #[serde(default)]
    pub next_entry_id: u64,
}

impl ChronicleKind {
    /// Parses a lowercase kind token into its chronicle kind.
    ///
    /// Rejects anything outside the fixed `fired|resolved|expired|founding`
    /// set so invalid bridge filters error instead of silently matching
    /// nothing. Shared by every scripting bridge.
    pub fn parse(raw: &str) -> Option<ChronicleKind> {
        match raw {
            "fired" => Some(ChronicleKind::Fired),
            "resolved" => Some(ChronicleKind::Resolved),
            "expired" => Some(ChronicleKind::Expired),
            "founding" => Some(ChronicleKind::Founding),
            _ => None,
        }
    }
}

/// Appends one chronicle entry to a lore store, minting its id.
///
/// State-level core behind [`append_chronicle_entry`] and the WASM world
/// mirror: one monotonic id sequence per store, never reused.
pub(crate) fn append_chronicle_entry_in(
    state: &mut LoreState,
    turn: u64,
    scenario_id: &str,
    kind: ChronicleKind,
    summary: String,
    choice_id: Option<String>,
) -> u64 {
    let entry_id = state.next_entry_id;
    state.next_entry_id += 1;
    state.entries.push(ChronicleEntry {
        entry_id,
        turn,
        scenario_id: scenario_id.to_string(),
        kind,
        summary,
        choice_id,
    });
    entry_id
}

/// Appends one chronicle entry, minting its id from the world counter.
///
/// Returns the minted entry id. Ids increase monotonically and are never
/// reused, including across save/load round-trips.
pub fn append_chronicle_entry(
    world: &mut World,
    turn: u64,
    scenario_id: &str,
    kind: ChronicleKind,
    summary: String,
    choice_id: Option<String>,
) -> u64 {
    append_chronicle_entry_in(&mut world.lore, turn, scenario_id, kind, summary, choice_id)
}

/// Returns the number of chronicle entries on the world.
pub fn chronicle_len(world: &World) -> usize {
    world.lore.entries.len()
}

/// Lists chronicle entries in a lore store matching every set filter field.
///
/// State-level core behind [`list_chronicle`] and the WASM world mirror.
/// Conjunctive over scenario id, kind, and the inclusive turn range; unset
/// fields are unbounded. Results always sort by turn then entry id, so
/// backfilled founding entries (older turns, newer ids) slot before live
/// entries regardless of insertion order.
pub(crate) fn list_chronicle_in(state: &LoreState, filter: ChronicleFilter) -> Vec<ChronicleEntry> {
    let mut out: Vec<ChronicleEntry> = state
        .entries
        .iter()
        .filter(|entry| {
            filter
                .scenario_id
                .as_ref()
                .is_none_or(|wanted| *wanted == entry.scenario_id)
                && filter.kind.is_none_or(|wanted| wanted == entry.kind)
                && filter.turn_from.is_none_or(|from| entry.turn >= from)
                && filter.turn_to.is_none_or(|to| entry.turn <= to)
        })
        .cloned()
        .collect();
    out.sort_by_key(|entry| (entry.turn, entry.entry_id));
    out
}

/// Lists chronicle entries matching every set filter field.
///
/// Conjunctive over scenario id, kind, and the inclusive turn range; unset
/// fields are unbounded. Results always sort by turn then entry id, so
/// backfilled founding entries (older turns, newer ids) slot before live
/// entries regardless of insertion order.
pub fn list_chronicle(world: &World, filter: ChronicleFilter) -> Vec<ChronicleEntry> {
    list_chronicle_in(&world.lore, filter)
}

/// Returns the chronicle entry in a lore store for an id, if present.
///
/// State-level core behind [`get_chronicle_entry`] and the WASM world
/// mirror. Unknown ids return `None` and never error.
pub(crate) fn get_chronicle_entry_in(state: &LoreState, entry_id: u64) -> Option<ChronicleEntry> {
    state
        .entries
        .iter()
        .find(|entry| entry.entry_id == entry_id)
        .cloned()
}

/// Returns the chronicle entry for an id, if present.
///
/// Unknown ids return `None` and never error, matching the bridge contract.
pub fn get_chronicle_entry(world: &World, entry_id: u64) -> Option<ChronicleEntry> {
    get_chronicle_entry_in(&world.lore, entry_id)
}

/// Maps a narrative lifecycle transition onto its chronicle kind.
fn chronicle_kind_of(kind: NarrativeRecordKind) -> ChronicleKind {
    match kind {
        NarrativeRecordKind::Fired => ChronicleKind::Fired,
        NarrativeRecordKind::Resolved => ChronicleKind::Resolved,
        NarrativeRecordKind::Expired => ChronicleKind::Expired,
    }
}

/// Lowercase kind token for the fixed chronicle template.
fn kind_label(kind: ChronicleKind) -> &'static str {
    match kind {
        ChronicleKind::Fired => "fired",
        ChronicleKind::Resolved => "resolved",
        ChronicleKind::Expired => "expired",
        ChronicleKind::Founding => "founding",
    }
}

/// Renders one chronicle summary through the fixed template.
///
/// `Turn {turn}: {scenario_name} — {kind} [{choice}]`, with the choice
/// bracket omitted when no choice resolved. The scenario name falls back to
/// the scenario id when metadata is absent. No wall-clock, randomness, or
/// locale-dependent formatting enters the output path.
fn render_summary(
    scenario_name: &str,
    turn: u64,
    kind: ChronicleKind,
    choice_id: Option<&str>,
) -> String {
    match choice_id {
        Some(choice) => format!(
            "Turn {turn}: {scenario_name} — {} [{choice}]",
            kind_label(kind)
        ),
        None => format!("Turn {turn}: {scenario_name} — {}", kind_label(kind)),
    }
}

/// Fixed code-side pool of founding-era names: slug plus display name.
/// Kept in code (not `scenarios.json`) so backfill determinism stays
/// self-contained in the persisted `LoreState` RNG stream. Backfill shuffles
/// this order per seed, then cycles for counts beyond eight.
const FOUNDING_ERAS: [(&str, &str); 8] = [
    ("first_hearth", "Founding of the First Hearth"),
    ("stone_circle", "Raising of the Stone Circle"),
    ("river_crossing", "Crossing of the Great River"),
    ("ashen_hollow", "Settlement of the Ashen Hollow"),
    ("high_meadow", "Claiming of the High Meadow"),
    ("deep_well", "Digging of the Deep Well"),
    ("ember_watch", "Lighting of the Ember Watch"),
    ("quiet_orchard", "Planting of the Quiet Orchard"),
];

/// Looks up the display name for a `founding:{slug}` scenario id.
fn founding_era_name(scenario_id: &str) -> Option<&'static str> {
    scenario_id
        .strip_prefix("founding:")
        .and_then(|slug| FOUNDING_ERAS.iter().find(|(known, _)| *known == slug))
        .map(|(_, name)| *name)
}

/// Resolves the display name for any chronicle scenario id against a
/// scenario table: registered metadata first, then the founding-era table,
/// then the raw id. Never fails; unknown ids fall back to the id itself.
///
/// State-level core behind [`resolve_scenario_name`] and the WASM world
/// mirror, which holds its own scenario table on `WasmWorld.narrative`.
pub(crate) fn resolve_scenario_name_in(
    scenarios: &HashMap<String, ScenarioDef>,
    scenario_id: &str,
) -> String {
    if let Some(def) = scenarios.get(scenario_id) {
        return def.name.clone();
    }
    if let Some(era) = founding_era_name(scenario_id) {
        return era.to_string();
    }
    scenario_id.to_string()
}

/// Renders a chronicle query from a lore store as human-readable text.
///
/// State-level core behind [`render_chronicle_text`] and the WASM world
/// mirror. Each line re-derives through the fixed template in
/// [`list_chronicle_in`] order.
pub(crate) fn render_chronicle_text_in(
    state: &LoreState,
    scenarios: &HashMap<String, ScenarioDef>,
    filter: ChronicleFilter,
) -> Vec<String> {
    list_chronicle_in(state, filter)
        .iter()
        .map(|entry| {
            render_summary(
                &resolve_scenario_name_in(scenarios, &entry.scenario_id),
                entry.turn,
                entry.kind,
                entry.choice_id.as_deref(),
            )
        })
        .collect()
}

/// Renders the chronicle query as human-readable text, one line per entry.
///
/// Each line re-derives through the fixed template in
/// [`list_chronicle`] order, so renaming a scenario (or backfilling a new
/// era) is reflected on the next render with no stored-state migration.
pub fn render_chronicle_text(world: &World, filter: ChronicleFilter) -> Vec<String> {
    render_chronicle_text_in(&world.lore, &world.narrative.scenarios, filter)
}

/// Populates founding-era entries in a lore store.
///
/// State-level core behind [`generate_founding_history`] and the WASM world
/// mirror. Appends exactly `era_count` founding entries (one per era, turns
/// `0..era_count` strictly increasing). `era_count == 0` returns `Ok(0)` and
/// mutates nothing. The evolved stream persists back into
/// `state.rng_state`; no wall-clock, thread RNG, or map-ordered iteration
/// enters the path.
pub(crate) fn generate_founding_history_in(
    state: &mut LoreState,
    seed: u64,
    era_count: u32,
) -> Result<usize, LoreError> {
    if era_count == 0 {
        return Ok(0);
    }
    let mut stream = [0u8; 32];
    stream[..8].copy_from_slice(&seed.to_le_bytes());
    let mut rng = SmallRng::from_seed(stream);
    let mut order = [0usize, 1, 2, 3, 4, 5, 6, 7];
    for i in (1..order.len()).rev() {
        let j = (rng.next_u32() as usize) % (i + 1);
        order.swap(i, j);
    }
    let count = era_count as usize;
    for i in 0..count {
        let (slug, name) = FOUNDING_ERAS[order[i % order.len()]];
        let scenario_id = format!("founding:{slug}");
        let turn = i as u64;
        let summary = render_summary(name, turn, ChronicleKind::Founding, None);
        append_chronicle_entry_in(
            state,
            turn,
            &scenario_id,
            ChronicleKind::Founding,
            summary,
            None,
        );
    }
    rng.fill(&mut stream);
    state.rng_state = stream;
    Ok(count)
}

/// Populates founding-era entries dated before live play.
///
/// Appends exactly `era_count` founding entries (one per era, turns
/// `0..era_count` strictly increasing), so they predate live play whenever
/// the world turn is 0 (founding before play) or exceeds `era_count`.
/// Callers backfilling a mid-game world with `era_count` at or above the
/// current turn should advance the turn first. `era_count == 0` returns
/// `Ok(0)` and mutates nothing.
///
/// The `seed` parameter fully determines the era order (shuffled per seed,
/// cycling past eight), so repeats with the same seed stay byte-identical,
/// including after [`clear_lore_history`]. The evolved stream persists back
/// into `lore.rng_state` following the narrative persist-back pattern; no
/// wall-clock, thread RNG, or map-ordered iteration enters the path.
pub fn generate_founding_history(
    world: &mut World,
    seed: u64,
    era_count: u32,
) -> Result<usize, LoreError> {
    generate_founding_history_in(&mut world.lore, seed, era_count)
}

/// Mirrors one narrative history record into a lore store.
///
/// State-level core behind [`mirror_narrative_record`] and the WASM world
/// mirror. Entry ids mint from the same counter as direct appends, keeping
/// one monotonic sequence per store.
pub(crate) fn mirror_record_in(
    state: &mut LoreState,
    scenarios: &HashMap<String, ScenarioDef>,
    record: &NarrativeRecord,
) {
    let kind = chronicle_kind_of(record.kind);
    let scenario_name = resolve_scenario_name_in(scenarios, &record.scenario_id);
    let summary = render_summary(
        &scenario_name,
        record.turn,
        kind,
        record.choice_id.as_deref(),
    );
    append_chronicle_entry_in(
        state,
        record.turn,
        &record.scenario_id,
        kind,
        summary,
        record.choice_id.clone(),
    );
}

/// Mirrors one narrative history record into the chronicle.
///
/// Called from the world-level narrative transitions (`tick_narrative` for
/// fired/expired records, `resolve_decision` for resolved records) so every
/// record is queryable no later than the end of its own tick. Entry ids mint
/// from the same counter as direct appends, keeping one monotonic sequence.
pub(crate) fn mirror_narrative_record(world: &mut World, record: &NarrativeRecord) {
    mirror_record_in(&mut world.lore, &world.narrative.scenarios, record);
}

/// Clears the entries of a lore store and resets its id counter.
///
/// State-level core behind [`clear_lore_history`] and the WASM world mirror.
pub(crate) fn clear_lore_history_in(state: &mut LoreState) {
    state.entries.clear();
    state.next_entry_id = 0;
}

/// Clears all chronicle entries and resets the id counter.
///
/// Test and regen support: mods call this before re-running seeded backfill
/// generation to reproduce byte-identical chronicles.
pub fn clear_lore_history(world: &mut World) {
    clear_lore_history_in(&mut world.lore);
}
