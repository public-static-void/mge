use super::{MapLink, World};
use crate::map::CellKey;
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

    /// Merge a patch into the cell metadata, preserving existing keys.
    ///
    /// If no map is present, this is a no-op. This is the safe write path for
    /// systems like fluid simulation that must not clobber existing metadata
    /// (`walkable`, `transparent`, `terrain`, etc.).
    pub fn merge_cell_metadata(&mut self, cell: &crate::map::CellKey, patch: serde_json::Value) {
        if let Some(map) = &mut self.map {
            map.merge_cell_metadata(cell, patch);
        }
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

    /// Link a source map cell to a target map cell. Errors on unknown map name.
    ///
    /// The link is stored per source map (a map's exit target). Both cells are
    /// topology-generic [`CellKey`]s, so any two topologies may be linked.
    pub fn link_maps(
        &mut self,
        source_map: &str,
        source_cell: CellKey,
        target_map: &str,
        target_cell: CellKey,
    ) -> Result<(), String> {
        if !self.maps.contains_key(source_map) {
            return Err(format!("Map '{source_map}' is not registered"));
        }
        if !self.maps.contains_key(target_map) {
            return Err(format!("Map '{target_map}' is not registered"));
        }
        self.map_links.insert(
            source_map.to_string(),
            MapLink {
                target_map: target_map.to_string(),
                source_cell,
                target_cell,
            },
        );
        Ok(())
    }

    /// Transition: set the active map to `name` and position the camera at `entry_cell`.
    ///
    /// Pushes the current active map onto the map stack (so `exit_map()` can
    /// return to it). Errors on unknown map name.
    pub fn enter_map(&mut self, name: &str, entry_cell: CellKey) -> Result<(), String> {
        let map = self
            .maps
            .get(name)
            .ok_or_else(|| format!("Map '{name}' is not registered"))?;
        if !self.active_map.is_empty() {
            self.map_stack.push(self.active_map.clone());
        }
        self.map = Some(map.clone());
        self.active_map = name.to_string();
        self.set_camera_position(&entry_cell);
        Ok(())
    }

    /// Transition: return to the previously active map (pop the map stack).
    ///
    /// Errors if the stack is empty (no prior map).
    pub fn exit_map(&mut self) -> Result<(), String> {
        let prev = self
            .map_stack
            .pop()
            .ok_or_else(|| "Cannot exit map: no previous map on the stack".to_string())?;
        let map = self
            .maps
            .get(&prev)
            .ok_or_else(|| format!("Map '{prev}' is not registered"))?;
        self.map = Some(map.clone());
        self.active_map = prev;
        Ok(())
    }

    /// Map a cell on `source_map` to the linked cell on the target map.
    ///
    /// Returns `None` if no link exists from that source cell.
    pub fn map_cell(&self, source_map: &str, source_cell: &CellKey) -> Option<CellKey> {
        let link = self.map_links.get(source_map)?;
        if &link.source_cell == source_cell {
            Some(link.target_cell.clone())
        } else {
            None
        }
    }

    /// Reverse mapping: map a cell on `target_map` back to the linked source cell.
    ///
    /// Returns `None` if no link exists to that target cell.
    pub fn unmap_cell(&self, target_map: &str, target_cell: &CellKey) -> Option<CellKey> {
        self.map_links
            .values()
            .find(|link| link.target_map == target_map && &link.target_cell == target_cell)
            .map(|link| link.source_cell.clone())
    }

    /// Position the camera entity at the given cell (find or create the camera entity).
    ///
    /// The camera is an entity with `Camera` + `Position` components, mirroring
    /// the bridge camera pattern. Province cells carry no x/y/z, so the `Camera`
    /// component falls back to zeros while the `Position` component keeps the
    /// province id.
    fn set_camera_position(&mut self, cell: &CellKey) {
        let pos_json = match cell {
            CellKey::Square { x, y, z } => {
                serde_json::json!({ "pos": { "Square": { "x": x, "y": y, "z": z } } })
            }
            CellKey::Hex { q, r, z } => {
                serde_json::json!({ "pos": { "Hex": { "q": q, "r": r, "z": z } } })
            }
            CellKey::Province { id } => {
                serde_json::json!({ "pos": { "Province": { "id": id } } })
            }
        };
        let (x, y, z) = match cell {
            CellKey::Square { x, y, z } => (*x, *y, *z),
            CellKey::Hex { q, r, z } => (*q, *r, *z),
            CellKey::Province { .. } => (0, 0, 0),
        };
        let camera_id = self
            .get_entities_with_component("Camera")
            .first()
            .cloned()
            .unwrap_or_else(|| self.spawn_entity());
        let _ = self.set_component(
            camera_id,
            "Camera",
            serde_json::json!({ "x": x, "y": y, "z": z }),
        );
        let _ = self.set_component(camera_id, "Position", pos_json);
    }
}
