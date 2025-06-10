use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::cell_key::CellKey;
use super::topology::MapTopology;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionMap {
    pub cells: HashMap<String, HashSet<String>>,
    pub cell_metadata: HashMap<String, Value>,
}

impl RegionMap {
    /// Create a new empty map
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            cell_metadata: HashMap::new(),
        }
    }

    /// Add a cell
    pub fn add_cell(&mut self, id: &str) {
        self.cells.entry(id.to_string()).or_default();
    }

    /// Add a neighbor
    pub fn add_neighbor(&mut self, from: &str, to: &str) {
        self.cells
            .entry(from.to_string())
            .or_default()
            .insert(to.to_string());
    }

    /// Merge another RegionMap into this one
    pub fn merge_from(&mut self, other: &RegionMap) {
        for (id, neighbors) in &other.cells {
            self.cells
                .entry(id.clone())
                .or_default()
                .extend(neighbors.iter().cloned());
        }
        for (id, meta) in &other.cell_metadata {
            self.cell_metadata
                .entry(id.clone())
                .or_insert_with(|| meta.clone());
        }
    }
}

impl Default for RegionMap {
    fn default() -> Self {
        Self::new()
    }
}

impl MapTopology for RegionMap {
    /// Returns the neighbors of a cell
    fn neighbors(&self, cell: &CellKey) -> Vec<CellKey> {
        if let CellKey::Region { id } = cell {
            self.cells
                .get(id)
                .map(|set| {
                    set.iter()
                        .map(|nid| CellKey::Region { id: nid.clone() })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            vec![]
        }
    }

    /// Returns true if the map contains the cell
    fn contains(&self, cell: &CellKey) -> bool {
        matches!(cell, CellKey::Region { id } if self.cells.contains_key(id))
    }

    /// Returns all the cells
    fn all_cells(&self) -> Vec<CellKey> {
        self.cells
            .keys()
            .map(|id| CellKey::Region { id: id.clone() })
            .collect()
    }

    /// Returns the type of the topology
    fn topology_type(&self) -> &'static str {
        "region"
    }

    /// Returns a reference to the topology
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns a mutable reference to the topology
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Sets the metadata for a cell
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value) {
        if let CellKey::Region { id } = cell {
            self.cell_metadata.insert(id.clone(), data);
        }
    }

    /// Gets the metadata for a cell
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value> {
        if let CellKey::Region { id } = cell {
            self.cell_metadata.get(id)
        } else {
            None
        }
    }
}
