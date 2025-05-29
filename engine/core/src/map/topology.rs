use serde_json::Value;
use std::any::Any;

use super::cell_key::CellKey;

pub trait MapTopology: Send + Sync {
    fn neighbors(&self, cell: &CellKey) -> Vec<CellKey>;
    fn contains(&self, cell: &CellKey) -> bool;
    fn all_cells(&self) -> Vec<CellKey>;
    fn topology_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value);
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value>;
}
