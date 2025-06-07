use super::World;
use crate::map::Map;
use serde_json::Value as JsonValue;
use std::sync::Arc;

impl World {
    /// Set metadata for a cell.
    pub fn set_cell_metadata(&mut self, cell: &crate::map::CellKey, data: serde_json::Value) {
        if let Some(map) = &mut self.map {
            map.set_cell_metadata(cell, data);
        }
    }

    /// Get metadata for a cell.
    pub fn get_cell_metadata(&self, cell: &crate::map::CellKey) -> Option<&serde_json::Value> {
        self.map.as_ref().and_then(|m| m.get_cell_metadata(cell))
    }

    /// Find path from start to goal using the world's map and cell metadata.
    pub fn find_path(
        &self,
        start: &crate::map::CellKey,
        goal: &crate::map::CellKey,
    ) -> Option<crate::map::pathfinding::PathfindingResult> {
        self.map.as_ref()?.find_path(start, goal)
    }

    /// Applies a generated map (from worldgen JSON) to the world and runs all postprocessors/validators.
    pub fn apply_generated_map(&mut self, map_json: &JsonValue) -> Result<(), String> {
        let map = Map::from_json(map_json)?;
        self.map = Some(map);

        let hooks = self.map_postprocessors.clone();
        for hook in hooks {
            hook(self)?; // If any returns Err, propagate immediately
        }

        Ok(())
    }

    /// Returns a reference to the world's map, if present.
    pub fn get_map(&self) -> Option<&Map> {
        self.map.as_ref()
    }

    /// Register a map postprocessor/validator hook.
    pub fn register_map_postprocessor<F>(&mut self, f: F)
    where
        F: Fn(&mut World) -> Result<(), String> + Send + Sync + 'static,
    {
        self.map_postprocessors.push(Arc::new(f));
    }

    /// Clear all map postprocessors.
    pub fn clear_map_postprocessors(&mut self) {
        self.map_postprocessors.clear();
    }
}
