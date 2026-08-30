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

    /// Applies a generated map after running all validators. Validators receive the map JSON.
    /// If any validator fails, returns an error and does not apply the map.
    pub fn apply_generated_map_with_validation(
        &mut self,
        map_json: &serde_json::Value,
    ) -> Result<(), String> {
        for validator in &self.map_validators {
            validator(map_json).map_err(|e| format!("Map validator failed: {e}"))?;
        }
        self.apply_generated_map(map_json)
            .map_err(|e| format!("Apply map failed: {e}"))?;
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

    /// Apply a map chunk (merge into the current map).
    pub fn apply_chunk(&mut self, chunk_json: &serde_json::Value) -> Result<(), String> {
        let chunk = Map::from_json(chunk_json)?;
        if let Some(ref mut map) = self.map {
            map.merge_chunk(&chunk);
        } else {
            self.map = Some(chunk);
        }
        Ok(())
    }

    /// Register a named map. Errors on duplicate name.
    pub fn register_map(&mut self, name: &str, map: Map) -> Result<(), String> {
        if self.maps.contains_key(name) {
            return Err(format!("Map '{name}' is already registered"));
        }
        self.maps.insert(name.to_string(), map);
        Ok(())
    }

    /// Set the active map to a registered map. Errors on unknown name.
    pub fn set_active_map(&mut self, name: &str) -> Result<(), String> {
        let map = self
            .maps
            .get(name)
            .ok_or_else(|| format!("Map '{name}' is not registered"))?;
        self.map = Some(map.clone());
        self.active_map = name.to_string();
        Ok(())
    }

    /// List all registered map names (unspecified order).
    pub fn get_map_names(&self) -> Vec<String> {
        self.maps.keys().cloned().collect()
    }

    /// Name of the current active map.
    pub fn get_active_map_name(&self) -> String {
        self.active_map.clone()
    }
}
