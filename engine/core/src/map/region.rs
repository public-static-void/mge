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
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            cell_metadata: HashMap::new(),
        }
    }
    pub fn add_cell(&mut self, id: &str) {
        self.cells.entry(id.to_string()).or_default();
    }
    pub fn add_neighbor(&mut self, from: &str, to: &str) {
        self.cells
            .entry(from.to_string())
            .or_default()
            .insert(to.to_string());
    }
}

impl Default for RegionMap {
    fn default() -> Self {
        Self::new()
    }
}

impl MapTopology for RegionMap {
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
    fn contains(&self, cell: &CellKey) -> bool {
        matches!(cell, CellKey::Region { id } if self.cells.contains_key(id))
    }
    fn all_cells(&self) -> Vec<CellKey> {
        self.cells
            .keys()
            .map(|id| CellKey::Region { id: id.clone() })
            .collect()
    }
    fn topology_type(&self) -> &'static str {
        "region"
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value) {
        if let CellKey::Region { id } = cell {
            self.cell_metadata.insert(id.clone(), data);
        }
    }
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value> {
        if let CellKey::Region { id } = cell {
            self.cell_metadata.get(id)
        } else {
            None
        }
    }
}
