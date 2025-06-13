use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::cell_key::CellKey;
use super::topology::MapTopology;

type CellSet = HashSet<CellKey>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquareGridMap {
    pub cells: HashMap<CellKey, CellSet>,
    pub cell_metadata: HashMap<CellKey, Value>,
}

impl SquareGridMap {
    /// Create a new empty map
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            cell_metadata: HashMap::new(),
        }
    }

    /// Add a cell to the map
    pub fn add_cell(&mut self, x: i32, y: i32, z: i32) {
        self.cells.entry(CellKey::Square { x, y, z }).or_default();
    }

    /// Add a neighbor to a cell
    pub fn add_neighbor(&mut self, from: (i32, i32, i32), to: (i32, i32, i32)) {
        self.cells
            .entry(CellKey::Square {
                x: from.0,
                y: from.1,
                z: from.2,
            })
            .or_default()
            .insert(CellKey::Square {
                x: to.0,
                y: to.1,
                z: to.2,
            });
    }

    /// Merge another SquareGridMap into this one
    pub fn merge_from(&mut self, other: &SquareGridMap) {
        for (cell, neighbors) in &other.cells {
            self.cells
                .entry(cell.clone())
                .or_default()
                .extend(neighbors.iter().cloned());
        }
        for (cell, meta) in &other.cell_metadata {
            self.cell_metadata
                .entry(cell.clone())
                .or_insert_with(|| meta.clone());
        }
    }
}

impl Default for SquareGridMap {
    fn default() -> Self {
        Self::new()
    }
}

impl MapTopology for SquareGridMap {
    /// Get the neighbors of a cell
    fn neighbors(&self, cell: &CellKey) -> Vec<CellKey> {
        if let CellKey::Square { .. } = cell {
            self.cells
                .get(cell)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            vec![]
        }
    }

    /// Check if the map contains a cell
    fn contains(&self, cell: &CellKey) -> bool {
        matches!(cell, CellKey::Square { .. } if self.cells.contains_key(cell))
    }

    /// Get all the cells in the map
    fn all_cells(&self) -> Vec<CellKey> {
        self.cells.keys().cloned().collect()
    }

    /// Get the topology type
    fn topology_type(&self) -> &'static str {
        "square"
    }

    /// Get a reference to the map
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Get a mutable reference to the map
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Set the metadata of a cell
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value) {
        self.cell_metadata.insert(cell.clone(), data);
    }

    /// Get the metadata of a cell
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value> {
        self.cell_metadata.get(cell)
    }
}
