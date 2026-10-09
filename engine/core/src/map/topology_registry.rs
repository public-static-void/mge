//! Topology construction/dispatch registry.
//!
//! Mirrors the [`crate::worldgen::MapgenAlgorithm`] precedent: adding a
//! topology is one [`register_topology`] call, with zero core `match` edits.
//! The [`CellKey`] enum itself is untouched — the registry covers
//! construction/dispatch only, so per-cell enum dispatch elsewhere stays.
//!
//! The JSON-transport side (`WasmMap`) keeps its string-key adapter and
//! shares only the pure helpers ([`cell_key_from_json`],
//! [`infer_neighbor_candidates`], [`default_cell`],
//! [`topology_matches_cell`]); the typed core is never forced across the
//! serialization boundary.

use super::{CellKey, Map};
use serde_json::Value;
use std::sync::{LazyLock, Mutex};

/// How a topology derives adjacency for cells without explicit neighbors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborStrategy {
    /// Infer 4-way (N/S/E/W) adjacency between present grid cells.
    FourWay,
    /// Infer 6-way axial adjacency between present hex cells.
    SixWay,
    /// Adjacency must be explicit; no inference.
    ExplicitOnly,
}

/// Which field-of-view algorithm a topology renders best with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FovKind {
    /// Grid shadowcasting for orthogonal maps.
    RecursiveShadowcasting,
    /// Flood fill for non-orthogonal or irregular maps.
    BfsFloodFill,
}

impl FovKind {
    /// Stable algorithm name shared with the FOV dispatch sites.
    pub fn algorithm_name(self) -> &'static str {
        match self {
            FovKind::RecursiveShadowcasting => "recursive_shadowcasting",
            FovKind::BfsFloodFill => "bfs_flood_fill",
        }
    }
}

/// One swappable topology: construction entry plus its dispatch metadata.
#[derive(Clone, Copy)]
pub struct TopologyEntry {
    /// Registry name matched against the map JSON `topology` field.
    pub name: &'static str,
    /// Builds a typed [`Map`] from validated map JSON.
    pub from_json: fn(&Value) -> Option<Map>,
    /// Neighbor-inference strategy for cells without explicit neighbors.
    pub neighbor_strategy: NeighborStrategy,
    /// Field-of-view algorithm this topology prefers.
    pub preferred_fov: FovKind,
}

/// Owned entry list; registration replaces by name so engine inits stay idempotent.
pub struct TopologyRegistry {
    entries: Vec<TopologyEntry>,
}

impl TopologyRegistry {
    /// Registry preloaded with the three in-core topologies.
    pub fn with_builtins() -> Self {
        Self {
            entries: vec![
                TopologyEntry {
                    name: "square",
                    from_json: super::deserialize::square_from_json,
                    neighbor_strategy: NeighborStrategy::FourWay,
                    preferred_fov: FovKind::RecursiveShadowcasting,
                },
                TopologyEntry {
                    name: "hex",
                    from_json: super::deserialize::hex_from_json,
                    neighbor_strategy: NeighborStrategy::SixWay,
                    preferred_fov: FovKind::BfsFloodFill,
                },
                TopologyEntry {
                    name: "province",
                    from_json: super::deserialize::province_from_json,
                    neighbor_strategy: NeighborStrategy::ExplicitOnly,
                    preferred_fov: FovKind::BfsFloodFill,
                },
            ],
        }
    }

    /// Register an entry, replacing any existing entry under the same name.
    pub fn register_or_replace(&mut self, entry: TopologyEntry) {
        self.entries.retain(|e| e.name != entry.name);
        self.entries.push(entry);
    }

    /// Copy of the entry registered under `name`, if any.
    pub fn resolve(&self, name: &str) -> Option<TopologyEntry> {
        self.entries.iter().find(|e| e.name == name).copied()
    }

    /// Names of all registered entries, in registration order.
    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.to_string()).collect()
    }
}

impl Default for TopologyRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

/// Process-wide topology table, following the worldgen global-registry shape.
/// Entries are construction metadata (function pointers + enums), so the
/// table grows only by explicit registration — no per-tick mutable state.
static TOPOLOGY_REGISTRY: LazyLock<Mutex<TopologyRegistry>> =
    LazyLock::new(|| Mutex::new(TopologyRegistry::with_builtins()));

fn lock_registry() -> std::sync::MutexGuard<'static, TopologyRegistry> {
    TOPOLOGY_REGISTRY
        .lock()
        .expect("topology registry lock is never poisoned by design")
}

/// Register a topology, replacing any existing entry under the same name.
pub fn register_topology(entry: TopologyEntry) {
    lock_registry().register_or_replace(entry);
}

/// Copy of the entry registered under `name`, if any.
pub fn resolve_topology(name: &str) -> Option<TopologyEntry> {
    lock_registry().resolve(name)
}

/// Names of all registered topologies, in registration order.
pub fn registered_topology_names() -> Vec<String> {
    lock_registry().names()
}

/// Build the [`CellKey`] for one map-JSON cell under the named topology.
/// Returns an error for unknown topologies instead of defaulting silently.
pub fn cell_key_from_json(topology: &str, cell: &Value) -> Result<CellKey, String> {
    match topology {
        "square" => Ok(CellKey::Square {
            x: cell.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            y: cell.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            z: cell.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        }),
        "hex" => Ok(CellKey::Hex {
            q: cell.get("q").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            r: cell.get("r").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            z: cell.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        }),
        "province" => Ok(CellKey::Province {
            id: cell
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Province cell missing 'id'".to_string())?
                .to_string(),
        }),
        other => Err(format!("Unknown topology '{other}'")),
    }
}

/// Adjacency candidates for neighbor inference under the named topology.
/// Only candidates matching the key's own variant are returned, so mixed
/// cell sets never infer cross-variant edges.
pub fn infer_neighbor_candidates(topology: &str, key: &CellKey) -> Vec<CellKey> {
    match (topology, key) {
        ("square", CellKey::Square { x, y, z }) => vec![
            CellKey::Square {
                x: x + 1,
                y: *y,
                z: *z,
            },
            CellKey::Square {
                x: x - 1,
                y: *y,
                z: *z,
            },
            CellKey::Square {
                x: *x,
                y: y + 1,
                z: *z,
            },
            CellKey::Square {
                x: *x,
                y: y - 1,
                z: *z,
            },
        ],
        ("hex", CellKey::Hex { q, r, z }) => vec![
            CellKey::Hex {
                q: q + 1,
                r: *r,
                z: *z,
            },
            CellKey::Hex {
                q: q - 1,
                r: *r,
                z: *z,
            },
            CellKey::Hex {
                q: *q,
                r: r + 1,
                z: *z,
            },
            CellKey::Hex {
                q: *q,
                r: r - 1,
                z: *z,
            },
            CellKey::Hex {
                q: q + 1,
                r: r - 1,
                z: *z,
            },
            CellKey::Hex {
                q: q - 1,
                r: r + 1,
                z: *z,
            },
        ],
        _ => Vec::new(),
    }
}

/// Blank cell for coordinate placement under the named topology.
/// Province cells are id-addressed rather than coordinate-addressed, so
/// there is no blank coordinate cell to create.
pub fn default_cell(topology: &str, x: i32, y: i32, z: i32) -> Option<CellKey> {
    match topology {
        "hex" => Some(CellKey::Hex { q: x, r: y, z }),
        "province" => None,
        _ => Some(CellKey::Square { x, y, z }),
    }
}

/// True when the cell variant matches the named topology.
pub fn topology_matches_cell(topology: &str, cell: &CellKey) -> bool {
    match cell {
        CellKey::Square { .. } => topology == "square",
        CellKey::Hex { .. } => topology == "hex",
        CellKey::Province { .. } => topology == "province",
    }
}
