use crate::map::{CellKey, MapTopology};
use serde_json::Value;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// A* pathfinding result
#[derive(Debug, Clone, PartialEq)]
pub struct PathfindingResult {
    /// Path from start to goal
    pub path: Vec<CellKey>,
    /// Total cost
    pub total_cost: f32,
}

#[derive(Debug, Clone)]
struct Node {
    cell: CellKey,
    estimate: f32,
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse for min-heap, handle NaN
        other
            .estimate
            .partial_cmp(&self.estimate)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.estimate == other.estimate
    }
}

impl Eq for Node {}

/// Default cost function: looks for "cost" in cell metadata, else returns 1.0.
/// If "walkable" is false, returns f32::INFINITY (impassable).
pub fn default_cost_fn(meta: Option<&Value>) -> f32 {
    if let Some(meta) = meta {
        if let Some(walkable) = meta.get("walkable")
            && walkable == &Value::Bool(false)
        {
            return f32::INFINITY;
        }
        if let Some(cost) = meta.get("cost")
            && let Some(c) = cost.as_f64()
        {
            return c as f32;
        }
    }
    1.0
}

/// Default heuristic: Manhattan distance for Square, 0 for others (Dijkstra fallback).
pub fn default_heuristic(a: &CellKey, b: &CellKey) -> f32 {
    match (a, b) {
        (
            CellKey::Square {
                x: ax,
                y: ay,
                z: az,
            },
            CellKey::Square {
                x: bx,
                y: by,
                z: bz,
            },
        ) => ((ax - bx).abs() + (ay - by).abs() + (az - bz).abs()) as f32,
        // For hex or province, fallback to 0 (Dijkstra)
        _ => 0.0,
    }
}

/// Generic A* pathfinding for any MapTopology.
/// - `cost_fn` is called with cell metadata (or None).
/// - `heuristic` is called with (current, goal) cell.
pub fn find_path<'a>(
    map: &dyn MapTopology,
    start: &CellKey,
    goal: &CellKey,
    cost_fn: &dyn Fn(Option<&Value>) -> f32,
    heuristic: &dyn Fn(&CellKey, &CellKey) -> f32,
    get_meta: &'a dyn Fn(&CellKey) -> Option<&'a Value>,
) -> Option<PathfindingResult> {
    if !map.contains(start) || !map.contains(goal) {
        return None;
    }

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<CellKey, CellKey> = HashMap::new();
    let mut g_score: HashMap<CellKey, f32> = HashMap::new();
    let mut closed: HashSet<CellKey> = HashSet::new();

    g_score.insert(start.clone(), 0.0);
    open.push(Node {
        cell: start.clone(),
        estimate: heuristic(start, goal),
    });

    while let Some(Node { cell, .. }) = open.pop() {
        if &cell == goal {
            // Reconstruct path
            let mut path = vec![cell.clone()];
            let mut current = cell;
            while let Some(prev) = came_from.get(&current) {
                path.push(prev.clone());
                current = prev.clone();
            }
            path.reverse();
            let total_cost = *g_score.get(goal).unwrap_or(&f32::INFINITY);
            return Some(PathfindingResult { path, total_cost });
        }

        if closed.contains(&cell) {
            continue;
        }
        closed.insert(cell.clone());

        for neighbor in map.neighbors(&cell) {
            if closed.contains(&neighbor) {
                continue;
            }
            let meta = get_meta(&neighbor);
            let step_cost = cost_fn(meta);
            if !step_cost.is_finite() {
                continue; // Impassable
            }
            let tentative_g = g_score.get(&cell).unwrap_or(&f32::INFINITY) + step_cost;
            if tentative_g < *g_score.get(&neighbor).unwrap_or(&f32::INFINITY) {
                came_from.insert(neighbor.clone(), cell.clone());
                g_score.insert(neighbor.clone(), tentative_g);
                let estimate = tentative_g + heuristic(&neighbor, goal);
                open.push(Node {
                    cell: neighbor,
                    estimate,
                });
            }
        }
    }
    None
}
