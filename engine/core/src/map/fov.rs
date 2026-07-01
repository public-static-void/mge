//! Field-of-view computation.
//!
//! Uses the [`FovAlgorithm`] trait to make FOV pluggable across map topologies:
//! - [`RecursiveShadowcasting`] — works on square grids (recursive symmetric shadowcasting)
//! - [`BfsFovAlgorithm`] — works on hex and province grids (BFS-based flood fill)
//!
//! Custom algorithms can be registered on the
//! [`World`](crate::ecs::world::World) via
//! [`set_fov_algorithm`](crate::ecs::world::World::set_fov_algorithm).
//!
//! Recursive shadowcasting algorithm from Albert Ford's "Symmetric Shadowcasting":
//! <https://www.albertford.com/shadowcasting/>
//!
//! Uses integer fractions for slope tracking — no floating point.
//! Processes 4 quadrants (north, south, east, west), scanning rows outward
//! and splitting the visible cone when walls are encountered.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

use super::cell_key::CellKey;
use super::topology::MapTopology;

/// A pluggable, topology-agnostic field-of-view algorithm.
///
/// Implementations use [`MapTopology::neighbors`] for graph traversal and
/// [`MapTopology::get_cell_metadata`] to determine which cells block line of
/// sight (cells with `"transparent": false` in metadata are opaque).
///
/// Implementations must be [`Send`] + [`Sync`] so they can be stored in the
/// ECS [`World`](crate::ecs::world::World) which is shared across threads.
pub trait FovAlgorithm: Send + Sync {
    /// Compute visible cells from an origin with given range.
    ///
    /// * `origin` — the observer's position
    /// * `range` — maximum sight distance in cells
    /// * `topology` — the map topology (provides neighbor traversal and metadata)
    ///
    /// Returns a list of visible [`CellKey`]s. The origin is always included.
    fn compute_fov(&self, origin: &CellKey, range: u32, topology: &dyn MapTopology)
    -> Vec<CellKey>;

    /// Human-readable name for debugging and API lookups.
    fn name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// Helpers shared by all algorithms
// ---------------------------------------------------------------------------

/// Check whether a cell is transparent (does not block line of sight).
/// Cells with no metadata default to transparent.
fn is_transparent(topology: &dyn MapTopology, cell: &CellKey) -> bool {
    topology
        .get_cell_metadata(cell)
        .and_then(|m| m.get("transparent"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

// ---------------------------------------------------------------------------
// Recursive symmetric shadowcasting (square grids only)
// ---------------------------------------------------------------------------

/// Recursive symmetric shadowcasting implementation.
///
/// This is the default FOV algorithm for square grids. Based on Albert Ford's
/// symmetric shadowcasting. Handles arbitrary wall configurations and produces
/// symmetric visibility.
///
/// Only works with [`CellKey::Square`] variants — returns empty for other
/// topologies.
pub struct RecursiveShadowcasting;

impl FovAlgorithm for RecursiveShadowcasting {
    fn compute_fov(
        &self,
        origin: &CellKey,
        range: u32,
        topology: &dyn MapTopology,
    ) -> Vec<CellKey> {
        let (ox, oy, oz) = match origin {
            CellKey::Square { x, y, z } => (*x, *y, *z),
            _ => return Vec::new(),
        };

        if range == 0 {
            return Vec::new();
        }
        let range = range as i32;
        let mut visible: HashSet<CellKey> = HashSet::new();

        // Origin is always visible
        visible.insert(origin.clone());

        // Build is_opaque from topology
        let is_opaque = |x: i32, y: i32| -> bool {
            let cell = CellKey::Square { x, y, z: oz };
            !topology.contains(&cell) || !is_transparent(topology, &cell)
        };

        // Scan each of the 4 quadrants (90-degree sectors)
        for &quadrant in &[NORTH, SOUTH, EAST, WEST] {
            let mut ctx = ScanContext {
                ox,
                oy,
                oz,
                range,
                quadrant,
                is_opaque: &is_opaque,
                visible: &mut visible,
            };
            ctx.scan(1, Slope::neg_one(), Slope::one());
        }

        visible.into_iter().collect()
    }

    fn name(&self) -> &'static str {
        "recursive_shadowcasting"
    }
}

// ---------------------------------------------------------------------------
// BFS FOV (any graph topology — hex and province grids)
// ---------------------------------------------------------------------------

/// BFS-based field-of-view for any graph topology.
///
/// Uses a breadth-first flood fill from the origin. Opaque cells block further
/// propagation but are themselves visible. Works for hex grids, province maps,
/// or any map topology that implements [`MapTopology::neighbors`].
///
/// Range is measured in graph-distance steps (edges traversed), which on a
/// regular hex grid corresponds to the hex distance.
pub struct BfsFovAlgorithm;

impl FovAlgorithm for BfsFovAlgorithm {
    fn compute_fov(
        &self,
        origin: &CellKey,
        range: u32,
        topology: &dyn MapTopology,
    ) -> Vec<CellKey> {
        if range == 0 {
            return Vec::new();
        }

        if !topology.contains(origin) {
            return Vec::new();
        }

        let range = range as i32;
        let mut visible: HashSet<CellKey> = HashSet::new();
        let mut queue: VecDeque<(CellKey, i32)> = VecDeque::new();
        let mut distances: HashMap<CellKey, i32> = HashMap::new();

        visible.insert(origin.clone());
        queue.push_back((origin.clone(), 0));
        distances.insert(origin.clone(), 0);

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= range {
                continue;
            }

            for neighbor in topology.neighbors(&current) {
                if !topology.contains(&neighbor) || visible.contains(&neighbor) {
                    continue;
                }

                let opaque = !is_transparent(topology, &neighbor);

                // Mark the cell as visible (walls are visible too)
                visible.insert(neighbor.clone());
                distances.insert(neighbor.clone(), depth + 1);

                // Only propagate through transparent cells (opaque blocks)
                if !opaque {
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        visible.into_iter().collect()
    }

    fn name(&self) -> &'static str {
        "bfs_flood_fill"
    }
}

// ---------------------------------------------------------------------------
// Internal helpers (shared by RecursiveShadowcasting)
// ---------------------------------------------------------------------------

/// Integer fraction for slope comparison.
/// Denominator is always positive after normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slope {
    num: i32,
    den: i32,
}

impl Slope {
    fn new(num: i32, den: i32) -> Self {
        if den < 0 {
            Slope {
                num: -num,
                den: -den,
            }
        } else {
            Slope { num, den }
        }
    }

    fn one() -> Self {
        Slope { num: 1, den: 1 }
    }

    fn neg_one() -> Self {
        Slope { num: -1, den: 1 }
    }
}

impl PartialOrd for Slope {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Both denominators are positive after normalization.
        // a/b > c/d  ⇔  a*d > c*b
        let lhs = self.num as i64 * other.den as i64;
        let rhs = other.num as i64 * self.den as i64;
        Some(lhs.cmp(&rhs))
    }
}

/// Round `n + 0.5` toward positive infinity (floor(n + 0.5) for all n).
fn round_ties_up(num: i32, den: i32) -> i32 {
    let n = 2 * num + den;
    let d = 2 * den;
    if n >= 0 { n / d } else { (n - d + 1) / d }
}

/// Round `n - 0.5` toward negative infinity (ceil(n - 0.5) for all n).
fn round_ties_down(num: i32, den: i32) -> i32 {
    let n = 2 * num - den;
    let d = 2 * den;
    if n >= 0 { (n + d - 1) / d } else { n / d }
}

/// Compute the slope of a tile's left edge: (2*col - 1) / (2*depth)
fn tile_slope(depth: i32, col: i32) -> Slope {
    Slope::new(2 * col - 1, 2 * depth)
}

// Quadrant transform directions
const NORTH: u8 = 0;
const SOUTH: u8 = 1;
const EAST: u8 = 2;
const WEST: u8 = 3;

/// Transform (depth, col) from quadrant-local to map (x, y) coordinates.
fn quadrant_transform(quadrant: u8, ox: i32, oy: i32, depth: i32, col: i32) -> (i32, i32) {
    match quadrant {
        NORTH => (ox + col, oy - depth),
        SOUTH => (ox + col, oy + depth),
        EAST => (ox + depth, oy + col),
        WEST => (ox - depth, oy + col),
        _ => unreachable!(),
    }
}

/// Check circular range constraint.
fn in_range(ox: i32, oy: i32, x: i32, y: i32, range: i32) -> bool {
    let dx = x - ox;
    let dy = y - oy;
    dx * dx + dy * dy <= range * range
}

/// Generate tiles for a row at `depth` bounded by `(start_slope, end_slope)`.
/// Returns a Vec of (col, slope_of_tile_left_edge) for tiles in this row.
fn row_tiles(depth: i32, start_slope: Slope, end_slope: Slope) -> Vec<(i32, Slope)> {
    // min_col = round_ties_up(depth * start_slope)
    let min_col = round_ties_up(depth * start_slope.num, start_slope.den);
    // max_col = round_ties_down(depth * end_slope)
    let max_col = round_ties_down(depth * end_slope.num, end_slope.den);
    (min_col..=max_col)
        .map(|col| (col, tile_slope(depth, col)))
        .collect()
}

/// Check if a floor tile's center (col, depth) is within the view cone.
fn is_symmetric(depth: i32, col: i32, start_slope: Slope, end_slope: Slope) -> bool {
    // col >= depth * start_slope  ⇔  col * start_slope.den >= depth * start_slope.num
    let above = col as i64 * start_slope.den as i64 >= depth as i64 * start_slope.num as i64;
    // col <= depth * end_slope  ⇔  col * end_slope.den <= depth * end_slope.num
    let below = col as i64 * end_slope.den as i64 <= depth as i64 * end_slope.num as i64;
    above && below
}

/// Context for a recursive shadowcasting scan.
///
/// Holds the invariant parameters (origin, range, quadrant, opacity test,
/// visible set) that are shared across all recursive calls. This reduces
/// the number of parameters passed to the scan function.
struct ScanContext<'a> {
    ox: i32,
    oy: i32,
    oz: i32,
    range: i32,
    quadrant: u8,
    is_opaque: &'a dyn Fn(i32, i32) -> bool,
    visible: &'a mut HashSet<CellKey>,
}

impl ScanContext<'_> {
    /// Recursively scan a row of tiles at `depth` bounded by the slope interval.
    fn scan(&mut self, depth: i32, start_slope: Slope, end_slope: Slope) {
        // Depth limit
        if depth > self.range {
            return;
        }

        let tiles = row_tiles(depth, start_slope, end_slope);
        let mut prev_tile_was_wall = false;
        let mut current_start = start_slope;

        for &(col, ts) in &tiles {
            let (x, y) = quadrant_transform(self.quadrant, self.ox, self.oy, depth, col);

            // Respect maximum range
            if !in_range(self.ox, self.oy, x, y, self.range) {
                continue;
            }

            let opaque = (self.is_opaque)(x, y);

            // Reveal wall tiles, and floor tiles whose centre is within the cone
            if opaque || is_symmetric(depth, col, current_start, end_slope) {
                self.visible.insert(CellKey::Square { x, y, z: self.oz });
            }

            // wall → floor transition: narrow the start_slope for subsequent children
            if prev_tile_was_wall && !opaque {
                current_start = ts;
            }

            // floor → wall transition: scan child with narrowed end_slope
            if !prev_tile_was_wall && opaque {
                self.scan(depth + 1, current_start, ts);
            }

            prev_tile_was_wall = opaque;
        }

        // Last tile was floor → continue scanning the next depth
        if !prev_tile_was_wall {
            self.scan(depth + 1, current_start, end_slope);
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience free function (backward-compatible wrapper)
// ---------------------------------------------------------------------------

/// Compute visible cells from an origin within a given range.
///
/// This convenience wrapper auto-selects the appropriate FOV algorithm based
/// on the map's topology type:
/// - `"square"` → [`RecursiveShadowcasting`]
/// - `"hex"` / `"province"` → [`BfsFovAlgorithm`]
/// - other → empty set
///
/// The origin cell is always included in the result.
pub fn compute_fov(map: &super::Map, origin: &CellKey, range: u32) -> HashSet<CellKey> {
    if range == 0 || !map.contains(origin) {
        return HashSet::new();
    }

    let visible: Vec<CellKey> = match map.topology_type() {
        "square" => RecursiveShadowcasting.compute_fov(origin, range, map.topology.as_ref()),
        "hex" | "province" => BfsFovAlgorithm.compute_fov(origin, range, map.topology.as_ref()),
        _ => return HashSet::new(),
    };

    let mut result: HashSet<CellKey> = visible.into_iter().collect();
    result.insert(origin.clone());
    result
}
