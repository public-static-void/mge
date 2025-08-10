use serde_json::Value;
use std::any::Any;

use super::cell_key::CellKey;

/// The topology of a map
pub trait MapTopology: Send + Sync {
    /// Returns the neighbors of a cell
    fn neighbors(&self, cell: &CellKey) -> Vec<CellKey>;
    /// Returns true if the cell is in the topology
    fn contains(&self, cell: &CellKey) -> bool;
    /// Returns all cells in the topology
    fn all_cells(&self) -> Vec<CellKey>;
    /// Returns the topology type
    fn topology_type(&self) -> &'static str;
    /// Returns the topology data
    fn as_any(&self) -> &dyn Any;
    /// Returns the topology data mutable
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Sets the cell metadata
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value);
    /// Gets the cell metadata
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value>;
}
