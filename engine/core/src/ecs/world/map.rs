use super::{HierarchyLevel, HierarchyReport, MAX_HIERARCHY_CELLS, MapLink, World, child_subseed};
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

    /// Get the fluid state for a cell, if any.
    ///
    /// Returns the `"fluid"` value from the cell's metadata — either the
    /// canonical single-type form `{"type": ..., "level": ...}` or the dual form
    /// `{"water": ..., "magma": ...}`. Returns `None` when the cell has no
    /// metadata or no `"fluid"` key.
    pub fn get_fluid(&self, cell: &crate::map::CellKey) -> Option<&serde_json::Value> {
        self.get_cell_metadata(cell)?.get("fluid")
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
    /// Links are keyed per `(source_map, source_cell)`, so a parent map
    /// accumulates many outbound links, one per distinct source cell, at every
    /// hierarchy level. Both cells are topology-generic [`CellKey`]s, so any two
    /// topologies may be linked.
    ///
    /// Re-linking an already-linked source cell with the **identical** target is
    /// an idempotent `Ok`. Linking it to any **different** target returns `Err`
    /// and leaves the prior link unchanged (no silent overwrite).
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
        let candidate = MapLink {
            target_map: target_map.to_string(),
            source_cell: source_cell.clone(),
            target_cell,
        };
        let source_links = self.map_links.entry(source_map.to_string()).or_default();
        match source_links.get(&source_cell) {
            Some(existing) if existing == &candidate => Ok(()),
            Some(existing) => Err(format!(
                "Map '{source_map}' cell {source_cell:?} is already linked to map '{}' cell {:?}",
                existing.target_map, existing.target_cell
            )),
            None => {
                source_links.insert(source_cell, candidate);
                Ok(())
            }
        }
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
    /// Returns `None` if no link exists from exactly that `(source_map, source_cell)`.
    pub fn map_cell(&self, source_map: &str, source_cell: &CellKey) -> Option<CellKey> {
        self.map_links
            .get(source_map)?
            .get(source_cell)
            .map(|link| link.target_cell.clone())
    }

    /// Reverse mapping: map a target cell back to a source cell, deterministically.
    ///
    /// When several links share the same `(target_map, target_cell)` anchor, the
    /// result is the link whose `(source_map, source_cell)` is minimum under the
    /// documented total order: `source_map` in UTF-8 byte order, then the
    /// canonical source-cell-key string (`serde_json::to_string(&CellKey)`) in
    /// byte order. The result is stable across repeated calls and process runs —
    /// `HashMap` iteration order never affects it.
    ///
    /// Returns `None` if no link exists to that target cell.
    pub fn unmap_cell(&self, target_map: &str, target_cell: &CellKey) -> Option<CellKey> {
        self.map_links
            .iter()
            .flat_map(|(source_map, links)| links.values().map(move |link| (source_map, link)))
            .filter(|(_, link)| link.target_map == target_map && &link.target_cell == target_cell)
            .min_by(|(map_a, link_a), (map_b, link_b)| {
                (map_a.as_str(), canonical_cell_key(&link_a.source_cell))
                    .cmp(&(map_b.as_str(), canonical_cell_key(&link_b.source_cell)))
            })
            .map(|(_, link)| link.source_cell.clone())
    }

    /// Enumerate every outbound link from `source_map`.
    ///
    /// Links are returned in deterministic canonical source-cell-key byte order
    /// (the serialized `CellKey` shape), independent of `HashMap` iteration order.
    /// Returns an empty `Vec` for an unknown or link-less source map.
    pub fn links_from(&self, source_map: &str) -> Vec<MapLink> {
        let Some(links) = self.map_links.get(source_map) else {
            return Vec::new();
        };
        let mut out: Vec<MapLink> = links.values().cloned().collect();
        out.sort_by_cached_key(|link| canonical_cell_key(&link.source_cell));
        out
    }

    /// Generate and link a bounded hierarchy of child maps rooted at `root`.
    ///
    /// For each level in `levels` (in order), one child map is generated per cell
    /// of the current frontier (the root's cells at depth 0, then each generated
    /// child's cells at the next depth). Each child is registered under
    /// `"{parent}/L{level}/{cell_token}"` and its parent cell is linked to the
    /// child's topology-appropriate origin cell (`square`/`hex` `{0,0,0}`).
    ///
    /// Determinism (R007/NFR001): child names use a collision-free per-topology
    /// cell token; child seeds come from [`child_subseed`] over
    /// `(seed, level, parent cell)`; the frontier is processed in canonical
    /// `(parent name, cell key)` order, so `HashMap` iteration order never affects
    /// the output. The algorithm is resolved through the global worldgen registry.
    ///
    /// Bounded allocation (NFR002): the projected budget
    /// `root_cell_count * Π(width_i * height_i)` is computed up front and an `Err`
    /// is returned before any registration or link mutation when it exceeds
    /// [`MAX_HIERARCHY_CELLS`].
    ///
    /// `province`-topology children are out of generation scope (their cells carry
    /// no grid coordinates); a province frontier cell errors. All error paths
    /// return deterministic messages and never panic (NFR003).
    pub fn generate_hierarchy(
        &mut self,
        root: &str,
        seed: u64,
        levels: &[HierarchyLevel],
    ) -> Result<HierarchyReport, String> {
        let root_cells: Vec<CellKey> = {
            let root_map = self
                .maps
                .get(root)
                .ok_or_else(|| format!("Map '{root}' is not registered"))?;
            root_map.all_cells()
        };

        // Projected budget, validated before any mutation (NFR002).
        let mut projected: usize = root_cells.len();
        for (index, level) in levels.iter().enumerate() {
            if level.width <= 0 || level.height <= 0 {
                return Err(format!(
                    "Hierarchy level {index} has non-positive dimensions {}x{}",
                    level.width, level.height
                ));
            }
            let per_child = (level.width as usize).saturating_mul(level.height as usize);
            projected = projected
                .checked_mul(per_child)
                .ok_or_else(|| format!("Hierarchy cell budget overflows at level {index}"))?;
            if projected > MAX_HIERARCHY_CELLS {
                return Err(format!(
                    "Hierarchy projected {projected} cells exceeds MAX_HIERARCHY_CELLS \
                     ({MAX_HIERARCHY_CELLS}) at level {index}"
                ));
            }
        }

        let mut generated_maps: Vec<String> = Vec::new();
        let mut total_cells: usize = 0;
        // Frontier entries: `(parent map name, parent cell)`; the next level
        // generates one child per entry.
        let mut frontier: Vec<(String, CellKey)> = root_cells
            .iter()
            .map(|cell| (root.to_string(), cell.clone()))
            .collect();

        for (index, level) in levels.iter().enumerate() {
            let child_level = (index + 1) as u32;
            // Canonical order keeps generation deterministic despite HashMap order.
            frontier.sort_by_cached_key(|(name, cell)| (name.clone(), canonical_cell_key(cell)));

            let mut next_frontier: Vec<(String, CellKey)> = Vec::new();
            for (parent_name, parent_cell) in &frontier {
                let token = hierarchy_cell_token(parent_cell)?;
                let child_name = format!("{parent_name}/L{child_level}/{token}");
                if self.maps.contains_key(&child_name) {
                    return Err(format!(
                        "Generated child map '{child_name}' is already registered"
                    ));
                }
                let child_seed = child_subseed(seed, child_level, parent_cell);
                let params = serde_json::json!({
                    "seed": child_seed,
                    "width": level.width,
                    "height": level.height,
                });
                let child_json = invoke_mapgen_algorithm(&level.algorithm, &params)?;
                let child_map = Map::from_json(&child_json)?;
                let origin = hierarchy_origin_cell(child_map.topology_type())?;
                if !child_map.contains(&origin) {
                    return Err(format!(
                        "Generated child map '{child_name}' is missing its origin cell {origin:?}"
                    ));
                }
                let child_cells = child_map.all_cells();
                self.register_map(&child_name, child_map)?;
                self.link_maps(parent_name, parent_cell.clone(), &child_name, origin)?;
                total_cells += child_cells.len();
                generated_maps.push(child_name.clone());
                for cell in child_cells {
                    next_frontier.push((child_name.clone(), cell));
                }
            }
            frontier = next_frontier;
        }

        Ok(HierarchyReport {
            generated_maps,
            total_cells,
        })
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

/// Canonical, order-stable string form of a cell key — the serialized enum shape
/// (`{"Square":{"x":..,"y":..,"z":..}}`, `{"Hex":..}`, `{"Province":..}`).
///
/// Used as the deterministic ordering key for link enumeration and reverse
/// lookup: `serde_json`'s encoding is byte-stable across runs, unlike `HashMap`
/// iteration order.
fn canonical_cell_key(cell: &CellKey) -> String {
    serde_json::to_string(cell).unwrap_or_default()
}

/// Collision-free per-topology token encoding a parent cell into a child name.
///
/// Square cells render as `x{x}_y{y}_z{z}` and hex cells as `q{q}_r{r}_z{z}`;
/// the coordinate prefixes keep the encodings unambiguous. Province cells are out
/// of hierarchy generation scope.
fn hierarchy_cell_token(cell: &CellKey) -> Result<String, String> {
    match cell {
        CellKey::Square { x, y, z } => Ok(format!("x{x}_y{y}_z{z}")),
        CellKey::Hex { q, r, z } => Ok(format!("q{q}_r{r}_z{z}")),
        CellKey::Province { id } => Err(format!(
            "Province cell '{id}' is out of hierarchy generation scope (square/hex only)"
        )),
    }
}

/// Topology-appropriate origin cell of a generated child map: `{0,0,0}` for the
/// uniform grid topologies the helper generates. Non-grid topologies error.
fn hierarchy_origin_cell(topology: &str) -> Result<CellKey, String> {
    match topology {
        "square" => Ok(CellKey::Square { x: 0, y: 0, z: 0 }),
        "hex" => Ok(CellKey::Hex { q: 0, r: 0, z: 0 }),
        other => Err(format!(
            "Topology '{other}' has no supported origin cell for hierarchy generation \
             (square/hex only)"
        )),
    }
}

/// Resolve and run a registered map-generation algorithm through the global
/// worldgen registry — the same pipeline the `invoke_worldgen_plugin` path uses.
/// Maps every registry failure to a deterministic, algorithm-named message.
fn invoke_mapgen_algorithm(algorithm: &str, params: &JsonValue) -> Result<JsonValue, String> {
    let registry = crate::worldgen::GLOBAL_WORLDGEN_REGISTRY
        .lock()
        .map_err(|_| "Worldgen registry lock is poisoned".to_string())?;
    registry
        .invoke(algorithm, params)
        .map_err(|err| format!("Worldgen algorithm '{algorithm}' is unavailable or failed: {err}"))
}
