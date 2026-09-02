use serde_json::{Map as JsonMap, Value};
use std::any::Any;

use super::cell_key::CellKey;

/// Recursively merge `patch` into `existing`.
///
/// For each key in `patch`: if both existing and patch values are JSON objects,
/// merge recursively; otherwise replace the existing value with the patch value.
/// Keys in `existing` not present in `patch` are preserved.
pub fn deep_merge(existing: &Value, patch: &Value) -> Value {
    match (existing, patch) {
        (Value::Object(base), Value::Object(patch_map)) => {
            let mut result = JsonMap::new();
            // Copy all existing keys
            for (k, v) in base {
                result.insert(k.clone(), v.clone());
            }
            // Merge patch keys
            for (k, v) in patch_map {
                if let Some(existing_val) = result.get(k) {
                    // Both objects → recurse; otherwise replace
                    result.insert(k.clone(), deep_merge(existing_val, v));
                } else {
                    result.insert(k.clone(), v.clone());
                }
            }
            Value::Object(result)
        }
        // Non-object patch replaces entirely
        _ => patch.clone(),
    }
}

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
    /// Merges a patch into the cell metadata.
    ///
    /// For each key in `patch`: if both existing and patch values are JSON objects,
    /// merge recursively; otherwise replace. Existing keys not in `patch` are
    /// preserved. If no metadata exists, the result is `patch` itself.
    fn merge_cell_metadata(&mut self, cell: &CellKey, patch: Value);
    /// Returns a boxed clone of this topology (dyn-compatible clone).
    fn clone_box(&self) -> Box<dyn MapTopology>;
}

impl Clone for Box<dyn MapTopology> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
