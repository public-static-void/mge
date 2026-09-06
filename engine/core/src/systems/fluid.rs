//! Fluid simulation system.
//!
//! Simulates water and magma flow across map cells using level-based flooding.
//! All fluid state is stored in cell metadata under the `"fluid"` key and
//! written exclusively through `merge_cell_metadata` to preserve existing
//! metadata (`walkable`, `transparent`, `terrain`, etc.).

use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::map::CellKey;
use serde_json::{Value, json};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

/// Fluid level at which a cell becomes impassable and opaque.
///
/// When a cell's fluid level reaches this threshold, the fluid system writes
/// `walkable: false` and `transparent: false` into the cell's metadata. Values
/// below this threshold restore the pre-fluid values.
pub const FLUID_BLOCK_LEVEL: i64 = 3;

/// Maximum fluid level per cell (saturation).
///
/// Fluid cannot exceed this level. Flow calculations cap transfers so that the
/// receiving cell never exceeds this value.
pub const FLUID_MAX_LEVEL: i64 = 8;

/// Ticks a water cell must go without inflow before it becomes `stale`.
pub const STALE_AFTER_TICKS: u32 = 5;

/// Ticks a shallow, stale water cell must go without outflow before it becomes
/// `swampy`.
pub const SWAMPY_AFTER_TICKS: u32 = 10;

/// Water-type taxonomy for a water cell.
///
/// `fresh` = salinity 0.0, `brackish` = 0.5, `salt` = 1.0. Mixing recomputes
/// the type from continuous salinity (see [`mix_water_type`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaterType {
    #[default]
    Fresh,
    Brackish,
    Salt,
}

impl WaterType {
    /// Salinity contribution of this type (0.0 fresh, 0.5 brackish, 1.0 salt).
    pub fn salinity(self) -> f64 {
        match self {
            WaterType::Fresh => 0.0,
            WaterType::Brackish => 0.5,
            WaterType::Salt => 1.0,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            WaterType::Fresh => "fresh",
            WaterType::Brackish => "brackish",
            WaterType::Salt => "salt",
        }
    }

    pub fn from_name(s: &str) -> WaterType {
        match s {
            "brackish" => WaterType::Brackish,
            "salt" => WaterType::Salt,
            _ => WaterType::Fresh,
        }
    }
}

/// Depth level of a water cell.
///
/// Stored explicitly and independent of fluid level (a deep cell may hold a
/// low level and vice versa). Only shallow cells can become `swampy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DepthLevel {
    #[default]
    Shallow,
    Deep,
}

impl DepthLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            DepthLevel::Shallow => "shallow",
            DepthLevel::Deep => "deep",
        }
    }

    pub fn from_name(s: &str) -> DepthLevel {
        match s {
            "deep" => DepthLevel::Deep,
            _ => DepthLevel::Shallow,
        }
    }
}

/// Flow state of a water cell, driven by inflow/outflow history.
///
/// `flowing` after inflow, `stale` after [`STALE_AFTER_TICKS`] without inflow,
/// `swampy` when shallow and stale for [`SWAMPY_AFTER_TICKS`] without outflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlowState {
    #[default]
    Flowing,
    Stale,
    Swampy,
}

impl FlowState {
    pub fn as_str(self) -> &'static str {
        match self {
            FlowState::Flowing => "flowing",
            FlowState::Stale => "stale",
            FlowState::Swampy => "swampy",
        }
    }

    pub fn from_name(s: &str) -> FlowState {
        match s {
            "stale" => FlowState::Stale,
            "swampy" => FlowState::Swampy,
            _ => FlowState::Flowing,
        }
    }
}

/// Per-type fluid levels and water taxonomy in a single cell.
///
/// Taxonomy fields (`water_type`, `depth`, `flow_state`) describe the water
/// portion and are only meaningful when `water > 0`. Magma cells never carry
/// them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FluidState {
    water: i64,
    magma: i64,
    water_type: WaterType,
    depth: DepthLevel,
    flow_state: FlowState,
    ticks_without_inflow: u32,
}

impl FluidState {
    fn total(self) -> i64 {
        self.water + self.magma
    }

    fn is_empty(self) -> bool {
        self.water == 0 && self.magma == 0
    }
}

/// Recomputes a receiving cell's water type from continuous salinity after a
/// water transfer.
///
/// The receiver's pre-transfer water is all of its current type; the incoming
/// water is all of `incoming_type`. `salinity = (existing_volume * existing
/// salinity + incoming_volume * incoming salinity) / total_water`. Result:
/// `salt` if >= 0.75, `brackish` if >= 0.25, else `fresh`. Skipped when the
/// receiving cell has no water (division by zero).
fn mix_water_type(receiver: &mut FluidState, incoming_volume: i64, incoming_type: WaterType) {
    if receiver.water <= 0 {
        return;
    }
    let existing_volume = receiver.water - incoming_volume;
    let salinity = (existing_volume as f64 * receiver.water_type.salinity()
        + incoming_volume as f64 * incoming_type.salinity())
        / receiver.water as f64;
    receiver.water_type = if salinity >= 0.75 {
        WaterType::Salt
    } else if salinity >= 0.25 {
        WaterType::Brackish
    } else {
        WaterType::Fresh
    };
}

/// Advances a water cell's flow state based on whether it received inflow this
/// tick.
///
/// Inflow resets the counter and restores `flowing`. No inflow increments the
/// counter (saturating at [`SWAMPY_AFTER_TICKS`]); past [`STALE_AFTER_TICKS`]
/// the cell becomes `stale`, and a shallow stale cell past
/// [`SWAMPY_AFTER_TICKS`] becomes `swampy`. Cells without water are untouched.
fn update_flow_state(state: &mut FluidState, received_inflow: bool) {
    if state.water == 0 {
        return;
    }
    if received_inflow {
        state.ticks_without_inflow = 0;
        state.flow_state = FlowState::Flowing;
    } else {
        state.ticks_without_inflow = state
            .ticks_without_inflow
            .saturating_add(1)
            .min(SWAMPY_AFTER_TICKS);
        if state.ticks_without_inflow >= STALE_AFTER_TICKS {
            state.flow_state = FlowState::Stale;
        }
        if state.flow_state == FlowState::Stale
            && state.depth == DepthLevel::Shallow
            && state.ticks_without_inflow >= SWAMPY_AFTER_TICKS
        {
            state.flow_state = FlowState::Swampy;
        }
    }
}

/// Reads the fluid state from a cell's metadata.
///
/// Accepts both the canonical single-type form (`{"type": ..., "level": ...}`)
/// and the dual form (`{"water": ..., "magma": ...}`) used when both fluid
/// types coexist in one cell. Taxonomy fields (`water_type`, `depth`,
/// `flow_state`) are parsed when present and default otherwise, so existing
/// serialized forms without them continue to parse.
fn read_fluid(meta: &Value) -> FluidState {
    let Some(fluid) = meta.get("fluid") else {
        return FluidState::default();
    };
    let taxonomy = |state: FluidState| FluidState {
        water_type: fluid
            .get("water_type")
            .and_then(Value::as_str)
            .map(WaterType::from_name)
            .unwrap_or_default(),
        depth: fluid
            .get("depth")
            .and_then(Value::as_str)
            .map(DepthLevel::from_name)
            .unwrap_or_default(),
        flow_state: fluid
            .get("flow_state")
            .and_then(Value::as_str)
            .map(FlowState::from_name)
            .unwrap_or_default(),
        ticks_without_inflow: fluid
            .get("ticks_without_inflow")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        ..state
    };
    if let Some(level) = fluid.get("level").and_then(Value::as_i64) {
        match fluid.get("type").and_then(Value::as_str) {
            Some("water") => taxonomy(FluidState {
                water: level.max(0),
                magma: 0,
                ..Default::default()
            }),
            Some("magma") => FluidState {
                water: 0,
                magma: level.max(0),
                ..Default::default()
            },
            _ => FluidState::default(),
        }
    } else {
        taxonomy(FluidState {
            water: fluid
                .get("water")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0),
            magma: fluid
                .get("magma")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0),
            ..Default::default()
        })
    }
}

/// Writes the fluid state into a cell's metadata, replacing the `"fluid"`
/// sub-object entirely via read-modify-set.
///
/// A naive merge would preserve stale keys from a prior fluid form (e.g.,
/// dual-form `water`/`magma` keys surviving into the empty form). Reading
/// the current metadata, inserting the new value at the `"fluid"` key, and
/// writing back eliminates stale keys while preserving sibling metadata.
fn write_fluid(world: &mut World, cell: &CellKey, state: FluidState) {
    let fluid_value = if state.water > 0 && state.magma > 0 {
        json!({
            "water": state.water,
            "magma": state.magma,
            "water_type": state.water_type.as_str(),
            "depth": state.depth.as_str(),
            "flow_state": state.flow_state.as_str(),
            "ticks_without_inflow": state.ticks_without_inflow,
        })
    } else if state.water > 0 {
        json!({
            "type": "water",
            "level": state.water,
            "water_type": state.water_type.as_str(),
            "depth": state.depth.as_str(),
            "flow_state": state.flow_state.as_str(),
            "ticks_without_inflow": state.ticks_without_inflow,
        })
    } else if state.magma > 0 {
        json!({ "type": "magma", "level": state.magma })
    } else {
        json!({ "type": "water", "level": 0 })
    };
    let mut meta = world.get_cell_metadata(cell).cloned().unwrap_or(json!({}));
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("fluid".to_string(), fluid_value);
    }
    world.set_cell_metadata(cell, meta);
}

/// Returns the cell directly below `cell`, if the topology has z-levels.
///
/// Production maps define no cross-z edges, so the downward neighbor is
/// synthesized and validated with `contains()` by the caller.
fn cell_below(cell: &CellKey) -> Option<CellKey> {
    match cell {
        CellKey::Square { x, y, z } => Some(CellKey::Square {
            x: *x,
            y: *y,
            z: z - 1,
        }),
        CellKey::Hex { q, r, z } => Some(CellKey::Hex {
            q: *q,
            r: *r,
            z: z - 1,
        }),
        CellKey::Province { .. } => None,
    }
}

/// Deterministic sort key for fluid processing order.
///
/// Primary key z descending (lower z-levels are processed after higher ones
/// receive downward flow), secondary x/q ascending, tertiary y/r ascending.
/// Province cells (no x/y/z) sort last by id.
fn cell_sort_key(cell: &CellKey) -> (u8, Reverse<i32>, i32, i32, String) {
    match cell {
        CellKey::Square { x, y, z } => (0, Reverse(*z), *x, *y, String::new()),
        CellKey::Hex { q, r, z } => (0, Reverse(*z), *q, *r, String::new()),
        CellKey::Province { id } => (1, Reverse(0), 0, 0, id.clone()),
    }
}

/// Fluid simulation system (water, magma).
///
/// Runs each tick and applies, in deterministic sorted cell order: magma-water
/// interaction (steam), downward z-flow, then horizontal flooding. Derived
/// blocking keys (`walkable`/`transparent`) are written when a cell's fluid
/// level reaches [`FLUID_BLOCK_LEVEL`] and restored when it drops below.
#[derive(Default)]
pub struct FluidSimulationSystem {
    /// Pre-fluid `walkable`/`transparent` values per cell, captured the first
    /// time the system blocks a cell and used to restore them when the fluid
    /// level drops below [`FLUID_BLOCK_LEVEL`].
    pre_fluid_blocking: HashMap<CellKey, (Option<bool>, Option<bool>)>,
}

impl System for FluidSimulationSystem {
    fn name(&self) -> &'static str {
        "FluidSimulationSystem"
    }

    fn run(&mut self, world: &mut World) {
        // Collect and sort cells deterministically; no-op without a map.
        let cells: Vec<CellKey> = {
            let Some(map) = world.map.as_ref() else {
                return;
            };
            let mut cells = map.all_cells();
            cells.sort_by_key(cell_sort_key);
            cells
        };

        // No-op on maps without fluid cells.
        let has_fluid = {
            let Some(map) = world.map.as_ref() else {
                return;
            };
            cells.iter().any(|cell| {
                map.get_cell_metadata(cell)
                    .map(|meta| read_fluid(meta).total() > 0)
                    .unwrap_or(false)
            })
        };
        if !has_fluid {
            return;
        }

        // Cells that received net water inflow this tick, used to drive
        // flow-state transitions in a second pass after all transfers.
        let mut received_inflow: HashSet<CellKey> = HashSet::new();

        for cell in &cells {
            let mut state = world
                .get_cell_metadata(cell)
                .map(read_fluid)
                .unwrap_or_default();
            if state.is_empty() {
                continue;
            }

            // Magma-water interaction: both types react to produce steam.
            if state.water > 0 && state.magma > 0 {
                state.water -= 1;
                state.magma -= 1;
                let _ = world.send_event(
                    "steam",
                    json!({ "cell": cell, "water": state.water, "magma": state.magma }),
                );
            }

            // Downward z-flow before horizontal spreading.
            if let Some(below) = cell_below(cell) {
                let below_exists = world
                    .map
                    .as_ref()
                    .map(|map| map.contains(&below))
                    .unwrap_or(false);
                if below_exists {
                    let mut below_state = world
                        .get_cell_metadata(&below)
                        .map(read_fluid)
                        .unwrap_or_default();
                    let before = below_state;
                    let water_in = transfer_down(&mut state.water, &mut below_state.water);
                    transfer_down(&mut state.magma, &mut below_state.magma);
                    if water_in > 0 {
                        received_inflow.insert(below.clone());
                        mix_water_type(&mut below_state, water_in, state.water_type);
                    }
                    if below_state != before {
                        write_fluid(world, &below, below_state);
                    }
                }
            }

            // Horizontal flooding into lower-level neighbors. Neighbors are
            // sorted so the transfer order is independent of HashSet iteration.
            let mut neighbors: Vec<CellKey> = world
                .map
                .as_ref()
                .map(|map| map.neighbors(cell))
                .unwrap_or_default();
            neighbors.sort_by_key(cell_sort_key);
            for neighbor in neighbors {
                let neighbor_exists = world
                    .map
                    .as_ref()
                    .map(|map| map.contains(&neighbor))
                    .unwrap_or(false);
                if !neighbor_exists {
                    continue;
                }
                let mut neighbor_state = world
                    .get_cell_metadata(&neighbor)
                    .map(read_fluid)
                    .unwrap_or_default();
                let before = neighbor_state;
                let water_in = transfer_horizontal(&mut state.water, &mut neighbor_state.water);
                transfer_horizontal(&mut state.magma, &mut neighbor_state.magma);
                if water_in > 0 {
                    received_inflow.insert(neighbor.clone());
                    mix_water_type(&mut neighbor_state, water_in, state.water_type);
                }
                if neighbor_state != before {
                    write_fluid(world, &neighbor, neighbor_state);
                }
            }

            write_fluid(world, cell, state);
            self.update_blocking_keys(world, cell, state);
        }

        // Second pass: advance flow states from inflow history. Only cells
        // whose flow state actually changed are written (write-on-change guard).
        for cell in &cells {
            let mut state = world
                .get_cell_metadata(cell)
                .map(read_fluid)
                .unwrap_or_default();
            if state.water == 0 {
                continue;
            }
            let before = state;
            update_flow_state(&mut state, received_inflow.contains(cell));
            if state != before {
                write_fluid(world, cell, state);
            }
        }
    }
}

impl FluidSimulationSystem {
    /// Writes derived `walkable`/`transparent` keys for a cell based on its
    /// fluid level, restoring pre-fluid values when the level drops below
    /// [`FLUID_BLOCK_LEVEL`].
    fn update_blocking_keys(&mut self, world: &mut World, cell: &CellKey, state: FluidState) {
        if state.total() >= FLUID_BLOCK_LEVEL {
            if !self.pre_fluid_blocking.contains_key(cell) {
                let meta = world.get_cell_metadata(cell).cloned();
                let walkable = meta
                    .as_ref()
                    .and_then(|m| m.get("walkable"))
                    .and_then(Value::as_bool);
                let transparent = meta
                    .as_ref()
                    .and_then(|m| m.get("transparent"))
                    .and_then(Value::as_bool);
                self.pre_fluid_blocking
                    .insert(cell.clone(), (walkable, transparent));
            }
            world.merge_cell_metadata(cell, json!({ "walkable": false, "transparent": false }));
        } else if let Some((walkable, transparent)) = self.pre_fluid_blocking.remove(cell) {
            // Absent pre-fluid values default to walkable/transparent in the
            // pathfinding and FOV consumers, so restoring `true` is equivalent
            // to restoring absence and keeps all writes on the merge path.
            world.merge_cell_metadata(
                cell,
                json!({
                    "walkable": walkable.unwrap_or(true),
                    "transparent": transparent.unwrap_or(true),
                }),
            );
        }
    }
}

/// Transfers one unit of fluid downward, capped by the receiving cell's
/// remaining capacity. Falls back to horizontal spreading when the cell below
/// is full (transfer of 0). Returns the amount transferred.
fn transfer_down(level: &mut i64, below_level: &mut i64) -> i64 {
    if *level > 0 && *below_level < FLUID_MAX_LEVEL {
        let transfer = (*level).min(1).min(FLUID_MAX_LEVEL - *below_level);
        *level -= transfer;
        *below_level += transfer;
        transfer
    } else {
        0
    }
}

/// Transfers `min(1, floor((L - L')) / 2)` units from a higher-level cell to a
/// lower-level neighbor, capped by the neighbor's remaining capacity. Returns
/// the amount transferred.
fn transfer_horizontal(level: &mut i64, neighbor_level: &mut i64) -> i64 {
    if *neighbor_level < *level {
        let transfer = (*level - *neighbor_level) / 2;
        let transfer = transfer.min(1).min(FLUID_MAX_LEVEL - *neighbor_level);
        if transfer > 0 {
            *level -= transfer;
            *neighbor_level += transfer;
            transfer
        } else {
            0
        }
    } else {
        0
    }
}
