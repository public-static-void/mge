//! Procedural history and lore: world-level chronicle read-model.
//!
//! Holds the persistent [`LoreState`] store on [`World`](crate::ecs::world::World),
//! following the free-function accessor split of `narrative.rs`: module-level
//! state on [`World`] plus thin accessors, with no per-entity owner required.
//!
//! This milestone covers the store shape, persistence wiring, and the append
//! path with monotonic ids. The live narrative feed, filtered queries,
//! deterministic rendering, and seeded founding backfill arrive in later
//! milestones and build on this state.

use crate::ecs::world::World;
use crate::narrative::{NarrativeRecord, NarrativeRecordKind};
use serde::{Deserialize, Serialize};

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
    let entry_id = world.lore.next_entry_id;
    world.lore.next_entry_id += 1;
    world.lore.entries.push(ChronicleEntry {
        entry_id,
        turn,
        scenario_id: scenario_id.to_string(),
        kind,
        summary,
        choice_id,
    });
    entry_id
}

/// Returns the number of chronicle entries on the world.
pub fn chronicle_len(world: &World) -> usize {
    world.lore.entries.len()
}

/// Lists chronicle entries matching every set filter field.
///
/// Conjunctive over scenario id, kind, and the inclusive turn range; unset
/// fields are unbounded. Results always sort by turn then entry id, so
/// backfilled founding entries (older turns, newer ids) slot before live
/// entries regardless of insertion order.
pub fn list_chronicle(world: &World, filter: ChronicleFilter) -> Vec<ChronicleEntry> {
    let mut out: Vec<ChronicleEntry> = world
        .lore
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

/// Returns the chronicle entry for an id, if present.
///
/// Unknown ids return `None` and never error, matching the bridge contract.
pub fn get_chronicle_entry(world: &World, entry_id: u64) -> Option<ChronicleEntry> {
    world
        .lore
        .entries
        .iter()
        .find(|entry| entry.entry_id == entry_id)
        .cloned()
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

/// Mirrors one narrative history record into the chronicle.
///
/// Called from the world-level narrative transitions (`tick_narrative` for
/// fired/expired records, `resolve_decision` for resolved records) so every
/// record is queryable no later than the end of its own tick. Entry ids mint
/// from the same counter as direct appends, keeping one monotonic sequence.
pub(crate) fn mirror_narrative_record(world: &mut World, record: &NarrativeRecord) {
    let kind = chronicle_kind_of(record.kind);
    let scenario_name = world
        .narrative
        .scenarios
        .get(&record.scenario_id)
        .map(|def| def.name.clone())
        .unwrap_or_else(|| record.scenario_id.clone());
    let summary = render_summary(
        &scenario_name,
        record.turn,
        kind,
        record.choice_id.as_deref(),
    );
    append_chronicle_entry(
        world,
        record.turn,
        &record.scenario_id,
        kind,
        summary,
        record.choice_id.clone(),
    );
}

/// Clears all chronicle entries and resets the id counter.
///
/// Test and regen support: mods call this before re-running seeded backfill
/// generation to reproduce byte-identical chronicles.
pub fn clear_lore_history(world: &mut World) {
    world.lore.entries.clear();
    world.lore.next_entry_id = 0;
}
