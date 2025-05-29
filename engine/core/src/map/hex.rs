use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::cell_key::CellKey;
use super::topology::MapTopology;

type CellSet = HashSet<CellKey>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexGridMap {
    pub cells: HashMap<CellKey, CellSet>,
    pub cell_metadata: HashMap<CellKey, Value>,
}

impl HexGridMap {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            cell_metadata: HashMap::new(),
        }
    }
    pub fn add_cell(&mut self, q: i32, r: i32, z: i32) {
        self.cells.entry(CellKey::Hex { q, r, z }).or_default();
    }
    pub fn add_neighbor(&mut self, from: (i32, i32, i32), to: (i32, i32, i32)) {
        self.cells
            .entry(CellKey::Hex {
                q: from.0,
                r: from.1,
                z: from.2,
            })
            .or_default()
            .insert(CellKey::Hex {
                q: to.0,
                r: to.1,
                z: to.2,
            });
    }
}

impl Default for HexGridMap {
    fn default() -> Self {
        Self::new()
    }
}

impl MapTopology for HexGridMap {
    fn neighbors(&self, cell: &CellKey) -> Vec<CellKey> {
        if let CellKey::Hex { .. } = cell {
            self.cells
                .get(cell)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            vec![]
        }
    }
    fn contains(&self, cell: &CellKey) -> bool {
        matches!(cell, CellKey::Hex { .. } if self.cells.contains_key(cell))
    }
    fn all_cells(&self) -> Vec<CellKey> {
        self.cells.keys().cloned().collect()
    }
    fn topology_type(&self) -> &'static str {
        "hex"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value) {
        self.cell_metadata.insert(cell.clone(), data);
    }
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value> {
        self.cell_metadata.get(cell)
    }
}
