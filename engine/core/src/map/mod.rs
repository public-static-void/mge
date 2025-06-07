pub mod cell_key;
pub mod deserialize;
pub mod hex;
pub mod pathfinding;
pub mod region;
pub mod square;
pub mod topology;

pub use cell_key::CellKey;
pub use hex::HexGridMap;
pub use pathfinding::{PathfindingResult, find_path as pathfinding_find_path};
pub use region::RegionMap;
use serde_json::Value;
pub use square::SquareGridMap;
pub use topology::MapTopology;

/// The main Map type (boxed trait object for dynamic dispatch).
pub struct Map {
    pub topology: Box<dyn MapTopology>,
}

impl Map {
    /// Create a new Map.
    pub fn new(topology: Box<dyn MapTopology>) -> Self {
        Self { topology }
    }

    /// Deserialize a Map from a JSON value.
    pub fn from_json(value: &Value) -> Option<Self> {
        crate::map::deserialize::map_from_json(value)
    }

    /// Check if the Map contains a cell.
    pub fn contains(&self, cell: &CellKey) -> bool {
        self.topology.contains(cell)
    }

    /// Get the neighbors of a cell.
    pub fn neighbors(&self, cell: &CellKey) -> Vec<CellKey> {
        self.topology.neighbors(cell)
    }

    /// Get the topology type of the Map.
    pub fn topology_type(&self) -> &'static str {
        self.topology.topology_type()
    }

    /// Get all the cells in the Map.
    pub fn all_cells(&self) -> Vec<CellKey> {
        self.topology.all_cells()
    }

    /// Get the underlying MapTopology as a reference.
    pub fn as_any(&self) -> &dyn std::any::Any {
        self.topology.as_any()
    }

    /// Get the underlying MapTopology as a mutable reference.
    pub fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self.topology.as_any_mut()
    }

    /// Set cell metadata for the Map.
    pub fn set_cell_metadata(&mut self, cell: &CellKey, data: Value) {
        self.topology.set_cell_metadata(cell, data);
    }

    /// Get cell metadata for the Map.
    pub fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value> {
        self.topology.get_cell_metadata(cell)
    }

    /// Find the path between two cells.
    pub fn find_path(&self, start: &CellKey, goal: &CellKey) -> Option<PathfindingResult> {
        crate::map::pathfinding::find_path(
            self.topology.as_ref(),
            start,
            goal,
            &crate::map::pathfinding::default_cost_fn,
            &crate::map::pathfinding::default_heuristic,
            &|cell| self.get_cell_metadata(cell),
        )
    }
}
