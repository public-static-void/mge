//! Field-of-view computation using recursive symmetric shadowcasting.
//!
//! Algorithm from Albert Ford's "Symmetric Shadowcasting":
//! <https://www.albertford.com/shadowcasting/>
//!
//! Uses integer fractions for slope tracking — no floating point.
//! Processes 4 quadrants (north, south, east, west), scanning rows outward
//! and splitting the visible cone when walls are encountered.

use std::cmp::Ordering;
use std::collections::HashSet;

use super::cell_key::CellKey;
use super::Map;

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
    if n >= 0 {
        n / d
    } else {
        (n - d + 1) / d
    }
}

/// Round `n - 0.5` toward negative infinity (ceil(n - 0.5) for all n).
fn round_ties_down(num: i32, den: i32) -> i32 {
    let n = 2 * num - den;
    let d = 2 * den;
    if n >= 0 {
        (n + d - 1) / d
    } else {
        n / d
    }
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

/// Compute visible cells from an origin within a given range
/// using recursive symmetric shadowcasting.
///
/// * `map` — The game map
/// * `origin` — The observer's position
/// * `range` — Maximum sight distance in cells
///
/// Returns a `HashSet` of visible `CellKey`s. The origin cell is always included.
pub fn compute_fov(map: &Map, origin: &CellKey, range: u32) -> HashSet<CellKey> {
    let mut visible = HashSet::new();

    if range == 0 {
        return visible;
    }

    let origin_coords = match origin {
        CellKey::Square { x, y, z } => (*x, *y, *z),
        _ => return visible,
    };
    let (ox, oy, oz) = origin_coords;
    let range = range as i32;

    // Origin is always visible
    visible.insert(origin.clone());

    // Scan each of the 4 quadrants (90-degree sectors)
    for quadrant in &[NORTH, SOUTH, EAST, WEST] {
        scan(
            map,
            ox, oy, oz,
            range,
            *quadrant,
            &mut visible,
            1,
            Slope::neg_one(),
            Slope::one(),
        );
    }

    visible
}

/// Generate tiles for a row at `depth` bounded by `(start_slope, end_slope)`.
/// Returns a Vec of (col, slope_of_tile_left_edge) for tiles in this row.
fn row_tiles(depth: i32, start_slope: Slope, end_slope: Slope) -> Vec<(i32, Slope)> {
    // min_col = round_ties_up(depth * start_slope)
    let min_col = round_ties_up(depth * start_slope.num, start_slope.den);
    // max_col = round_ties_down(depth * end_slope)
    let max_col = round_ties_down(depth * end_slope.num, end_slope.den);
    (min_col..=max_col).map(|col| (col, tile_slope(depth, col))).collect()
}

/// Check if a floor tile's center (col, depth) is within the view cone.
fn is_symmetric(depth: i32, col: i32, start_slope: Slope, end_slope: Slope) -> bool {
    // col >= depth * start_slope  ⇔  col * start_slope.den >= depth * start_slope.num
    let above = col as i64 * start_slope.den as i64 >= depth as i64 * start_slope.num as i64;
    // col <= depth * end_slope  ⇔  col * end_slope.den <= depth * end_slope.num
    let below = col as i64 * end_slope.den as i64 <= depth as i64 * end_slope.num as i64;
    above && below
}

/// Recursively scan a row, splitting the visible cone when walls are hit.
///
/// Follows Albert Ford's symmetric shadowcasting. Key behaviour:
/// - `start_slope` may be updated (narrowed) during the wall→floor transition
/// - `floor→wall` creates a child scan with narrowed end_slope
/// - If the last tile in the row is floor, continue scanning the next depth
fn scan(
    map: &Map,
    ox: i32, oy: i32, oz: i32,
    range: i32,
    quadrant: u8,
    visible: &mut HashSet<CellKey>,
    depth: i32,
    start_slope: Slope,
    end_slope: Slope,
) {
    // Depth limit
    if depth > range {
        return;
    }

    let tiles = row_tiles(depth, start_slope, end_slope);
    let mut prev_tile_was_wall = false;
    let mut current_start = start_slope;

    for &(col, ts) in &tiles {
        let (x, y) = quadrant_transform(quadrant, ox, oy, depth, col);

        let cell = CellKey::Square { x, y, z: oz };

        // Skip out-of-bounds cells
        if !map.contains(&cell) {
            prev_tile_was_wall = false;
            continue;
        }

        // Respect maximum range
        if !in_range(ox, oy, x, y, range) {
            continue;
        }

        let is_wall_tile = !is_transparent(map, &cell);

        // Reveal wall tiles, and floor tiles whose centre is within the cone
        if is_wall_tile || is_symmetric(depth, col, current_start, end_slope) {
            visible.insert(cell.clone());
        }

        // wall → floor transition: narrow the start_slope for subsequent children
        if prev_tile_was_wall && !is_wall_tile {
            current_start = ts;
        }

        // floor → wall transition: scan child with narrowed end_slope
        if !prev_tile_was_wall && is_wall_tile {
            scan(
                map,
                ox, oy, oz,
                range,
                quadrant,
                visible,
                depth + 1,
                current_start,
                ts,  // narrowed end_slope = wall's left edge
            );
        }

        prev_tile_was_wall = is_wall_tile;
    }

    // Last tile was floor → continue scanning the next depth
    if !prev_tile_was_wall {
        scan(
            map,
            ox, oy, oz,
            range,
            quadrant,
            visible,
            depth + 1,
            current_start,
            end_slope,
        );
    }
}

/// Check whether a cell is transparent (does not block line of sight).
fn is_transparent(map: &Map, cell: &CellKey) -> bool {
    map.get_cell_metadata(cell)
        .and_then(|m| m.get("transparent"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}
