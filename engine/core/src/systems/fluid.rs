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
use std::collections::HashMap;

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

/// Per-type fluid levels in a single cell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FluidState {
    water: i64,
    magma: i64,
}

impl FluidState {
    fn total(self) -> i64 {
        self.water + self.magma
    }

    fn is_empty(self) -> bool {
        self.water == 0 && self.magma == 0
    }
}

/// Reads the fluid state from a cell's metadata.
///
/// Accepts both the canonical single-type form (`{"type": ..., "level": ...}`)
/// and the dual form (`{"water": ..., "magma": ...}`) used when both fluid
/// types coexist in one cell.
fn read_fluid(meta: &Value) -> FluidState {
    let Some(fluid) = meta.get("fluid") else {
        return FluidState::default();
    };
    if let Some(level) = fluid.get("level").and_then(Value::as_i64) {
        match fluid.get("type").and_then(Value::as_str) {
            Some("water") => FluidState {
                water: level.max(0),
                magma: 0,
            },
            Some("magma") => FluidState {
                water: 0,
                magma: level.max(0),
            },
            _ => FluidState::default(),
        }
    } else {
        FluidState {
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
        }
    }
}

/// Writes the fluid state into a cell's metadata via the merge helper.
///
/// The canonical single-type form is written when only one fluid type is
/// present; the dual form is written when both coexist. An empty state is
/// written as `{"type": "water", "level": 0}` (level 0 means no fluid).
fn write_fluid(world: &mut World, cell: &CellKey, state: FluidState) {
    let fluid_value = if state.water > 0 && state.magma > 0 {
        json!({ "water": state.water, "magma": state.magma })
    } else if state.water > 0 {
        json!({ "type": "water", "level": state.water })
    } else if state.magma > 0 {
        json!({ "type": "magma", "level": state.magma })
    } else {
        json!({ "type": "water", "level": 0 })
    };
    world.merge_cell_metadata(cell, json!({ "fluid": fluid_value }));
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
                    transfer_down(&mut state.water, &mut below_state.water);
                    transfer_down(&mut state.magma, &mut below_state.magma);
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
                transfer_horizontal(&mut state.water, &mut neighbor_state.water);
                transfer_horizontal(&mut state.magma, &mut neighbor_state.magma);
                if neighbor_state != before {
                    write_fluid(world, &neighbor, neighbor_state);
                }
            }

            write_fluid(world, cell, state);
            self.update_blocking_keys(world, cell, state);
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
/// is full (transfer of 0).
fn transfer_down(level: &mut i64, below_level: &mut i64) {
    if *level > 0 && *below_level < FLUID_MAX_LEVEL {
        let transfer = (*level).min(1).min(FLUID_MAX_LEVEL - *below_level);
        *level -= transfer;
        *below_level += transfer;
    }
}

/// Transfers `min(1, floor((L - L')) / 2)` units from a higher-level cell to a
/// lower-level neighbor, capped by the neighbor's remaining capacity.
fn transfer_horizontal(level: &mut i64, neighbor_level: &mut i64) {
    if *neighbor_level < *level {
        let transfer = (*level - *neighbor_level) / 2;
        let transfer = transfer.min(1).min(FLUID_MAX_LEVEL - *neighbor_level);
        if transfer > 0 {
            *level -= transfer;
            *neighbor_level += transfer;
        }
    }
}
