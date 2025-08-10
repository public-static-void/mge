//! Map module.
//!
//! This module provides the Map struct and related functionality for handling maps in the game.

/// Cell key module.
pub mod cell_key;
/// Map deserialization module.
pub mod deserialize;
/// Hex grid map module.
pub mod hex;
/// Map pathfinding module.
pub mod pathfinding;
/// Region map module.
pub mod region;
/// Square grid map module.
pub mod square;
/// Map topology module.
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
    /// The underlying MapTopology.
    pub topology: Box<dyn MapTopology>,
}

impl Map {
    /// Create a new Map.
    pub fn new(topology: Box<dyn MapTopology>) -> Self {
        Self { topology }
    }

    /// Deserialize a Map from a JSON value.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        crate::map::deserialize::map_from_json(value)
            .ok_or_else(|| "Map parse error: could not parse map from JSON".to_string())
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

    /// Merge another map (chunk) into this map.
    pub fn merge_chunk(&mut self, other: &Map) {
        if self.topology_type() == other.topology_type() {
            if let Some(this_sq) = self.topology.as_any_mut().downcast_mut::<SquareGridMap>() {
                if let Some(other_sq) = other.topology.as_any().downcast_ref::<SquareGridMap>() {
                    this_sq.merge_from(other_sq);
                }
            } else if let Some(this_hex) = self.topology.as_any_mut().downcast_mut::<HexGridMap>() {
                if let Some(other_hex) = other.topology.as_any().downcast_ref::<HexGridMap>() {
                    this_hex.merge_from(other_hex);
                }
            } else if let Some(this_region) = self.topology.as_any_mut().downcast_mut::<RegionMap>()
                && let Some(other_region) = other.topology.as_any().downcast_ref::<RegionMap>()
            {
                this_region.merge_from(other_region);
            }
        } else {
            println!("Topology types do not match; skipping merge.");
        }
    }
}
