use super::World;
use crate::ecs::components::position::{Position, PositionComponent};

impl World {
    /// Spawn a new entity
    pub fn spawn_entity(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(id);
        id
    }

    /// Despawn an entity
    pub fn despawn_entity(&mut self, entity: u32) {
        for comps in self.components.values_mut() {
            let _existed = comps.remove(&entity).is_some();
        }
        self.entities.retain(|&id| id != entity);
    }

    /// Checks if an entity exists
    pub fn entity_exists(&self, entity: u32) -> bool {
        let in_entities = self.entities.contains(&entity);
        let in_any_component = self
            .components
            .values()
            .any(|comp_map| comp_map.contains_key(&entity));
        in_entities || in_any_component
    }

    /// Get all entities
    pub fn get_entities(&self) -> Vec<u32> {
        self.entities.clone()
    }

    /// Get all entities with a given component
    pub fn get_entities_with_component(&self, name: &str) -> Vec<u32> {
        if !self.is_component_allowed_in_mode(name, &self.current_mode) {
            return vec![];
        }
        self.components
            .get(name)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Checks if an entity has a component
    pub fn has_component(&self, entity: u32, name: &str) -> bool {
        self.components
            .get(name)
            .is_some_and(|m| m.contains_key(&entity))
    }

    /// Get all entities with a given component
    pub fn get_entities_with_components(&self, names: &[&str]) -> Vec<u32> {
        if names.is_empty() {
            return self.entities.clone();
        }
        let allowed_names: Vec<&&str> = names
            .iter()
            .filter(|&&name| self.is_component_allowed_in_mode(name, &self.current_mode))
            .collect();
        if allowed_names.is_empty() {
            return vec![];
        }
        let mut sets: Vec<std::collections::HashSet<u32>> = allowed_names
            .iter()
            .filter_map(|&&name| self.components.get(name))
            .map(|comps| comps.keys().cloned().collect())
            .collect();
        if sets.is_empty() {
            return vec![];
        }
        let first = sets.pop().unwrap();
        sets.into_iter()
            .fold(first, |acc, set| acc.intersection(&set).cloned().collect())
            .into_iter()
            .collect()
    }

    /// Move an entity
    pub fn move_entity(&mut self, entity: u32, dx: f32, dy: f32) {
        if let Some(value) = self.get_component(entity, "Position").cloned()
            && let Ok(mut pos_comp) = serde_json::from_value::<PositionComponent>(value)
        {
            if let Position::Square { x, y, .. } = &mut pos_comp.pos {
                *x += dx as i32;
                *y += dy as i32;
            }
            let _ =
                self.set_component(entity, "Position", serde_json::to_value(&pos_comp).unwrap());
        }
    }

    /// Move an entity in three dimensions.
    ///
    /// Square positions shift `x`/`y`/`z` by `dx`/`dy`/`dz`; Hex positions shift
    /// `q`/`r`/`z` by `dx`/`dy`/`dz`. Province positions and entities without a
    /// Position are left unchanged. The full Position component is re-serialized,
    /// preserving all other fields, mirroring [`move_entity`](Self::move_entity).
    pub fn move_entity_3d(&mut self, entity: u32, dx: f32, dy: f32, dz: f32) {
        if let Some(value) = self.get_component(entity, "Position").cloned()
            && let Ok(mut pos_comp) = serde_json::from_value::<PositionComponent>(value)
        {
            match &mut pos_comp.pos {
                Position::Square { x, y, z } => {
                    *x += dx as i32;
                    *y += dy as i32;
                    *z += dz as i32;
                }
                Position::Hex { q, r, z } => {
                    *q += dx as i32;
                    *r += dy as i32;
                    *z += dz as i32;
                }
                Position::Province { .. } => {}
            }
            let _ =
                self.set_component(entity, "Position", serde_json::to_value(&pos_comp).unwrap());
        }
    }

    /// Damage an entity.
    ///
    /// When the entity has a Body component, damage is routed through PendingDamage
    /// for distribution by BodyPartDamageSystem. Otherwise falls back to direct
    /// Health subtraction for backward compatibility.
    pub fn damage_entity(&mut self, entity: u32, amount: f32) {
        if self.has_component(entity, "Body") {
            self.append_pending_damage(entity, amount as f64, None);
        } else if let Some(healths) = self.components.get_mut("Health")
            && let Some(value) = healths.get_mut(&entity)
            && let Some(obj) = value.as_object_mut()
            && let Some(current) = obj.get_mut("current")
            && let Some(cur_val) = current.as_f64()
        {
            *current = serde_json::json!((cur_val - amount as f64).max(0.0));
        }
    }

    /// Apply targeted damage to a specific body part.
    ///
    /// Appends a PendingDamage entry with the specified `part_name`.
    /// If the entity has no Body component, this is a no-op.
    pub fn damage_entity_part(&mut self, entity: u32, part_name: &str, amount: f32) {
        if self.has_component(entity, "Body") {
            self.append_pending_damage(entity, amount as f64, Some(part_name));
        }
    }

    /// Appends a damage entry to the entity's PendingDamage component.
    /// Creates the component if it doesn't exist.
    fn append_pending_damage(&mut self, entity: u32, amount: f64, target_part: Option<&str>) {
        use serde_json::json;

        let target_part_val = match target_part {
            Some(name) => json!(name),
            None => json!(null),
        };

        let damage_entry = json!({
            "amount": amount,
            "target_part": target_part_val
        });

        if let Some(pending) = self.components.get_mut("PendingDamage")
            && let Some(value) = pending.get_mut(&entity)
            && let Some(obj) = value.as_object_mut()
            && let Some(damages) = obj.get_mut("damages")
            && let Some(arr) = damages.as_array_mut()
        {
            arr.push(damage_entry);
        } else {
            let pending = json!({
                "damages": [damage_entry]
            });
            // Bypass schema validation for internal PendingDamage writes
            self.components
                .entry("PendingDamage".to_string())
                .or_default()
                .insert(entity, pending);
        }
    }

    /// Checks if an entity is alive
    pub fn is_entity_alive(&self, entity: u32) -> bool {
        if let Some(health) = self.get_component(entity, "Health") {
            health
                .get("current")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                > 0.0
        } else {
            false
        }
    }

    /// Counts the number of entities with a given type
    pub fn count_entities_with_type(&self, type_str: &str) -> usize {
        self.get_entities_with_component("Type")
            .into_iter()
            .filter(|&id| {
                self.get_component(id, "Type")
                    .and_then(|v| v.get("kind"))
                    .and_then(|k| k.as_str())
                    .map(|k| k == type_str)
                    .unwrap_or(false)
            })
            .count()
    }

    /// Returns all entity IDs in the given cell.
    ///
    /// Matches Square positions by exact `x,y,z` and Hex positions by exact
    /// `q,r,z`. Province cells use capacity-1 occupancy semantics: only
    /// entities carrying a `Building` or `ConstructionSite` component count
    /// as occupants of the province, so placement validation never falls
    /// through silently on Hex/Province queries.
    pub fn entities_in_cell(&self, cell: &crate::map::CellKey) -> Vec<u32> {
        self.entities
            .iter()
            .copied()
            .filter(|&eid| {
                let Some(val) = self.get_component(eid, "Position") else {
                    return false;
                };
                let Some(key) = crate::map::CellKey::from_position(val) else {
                    return false;
                };
                if key != *cell {
                    return false;
                }
                if matches!(cell, crate::map::CellKey::Province { .. }) {
                    return self.has_component(eid, "Building")
                        || self.has_component(eid, "ConstructionSite");
                }
                true
            })
            .collect()
    }

    /// Returns all entity IDs on the given z-level.
    ///
    /// Matches both Square and Hex positions. Province positions (which have no
    /// z concept) and entities without a Position never match.
    pub fn entities_in_zlevel(&self, z: i32) -> Vec<u32> {
        self.entities
            .iter()
            .copied()
            .filter(|&eid| {
                self.get_component(eid, "Position")
                    .and_then(|val| {
                        val.get("pos").and_then(|p| {
                            if let Some(obj) = p.as_object() {
                                if let Some(sq) = obj.get("Square") {
                                    let zval = sq.get("z")?.as_i64()? as i32;
                                    return Some(zval == z);
                                }
                                if let Some(hex) = obj.get("Hex") {
                                    let zval = hex.get("z")?.as_i64()? as i32;
                                    return Some(zval == z);
                                }
                            }
                            None
                        })
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns all cells (as serde_json::Value) assigned to the given region_id.
    ///
    /// A `RegionAssignment` whose `cell` is `{ "Region": { "id": R } }` is
    /// resolved recursively to the actual cells of region `R` (cells whose
    /// `region_id` includes `R`), so callers never receive the opaque `Region`
    /// JSON. A visited set guards against region-reference cycles. Zone ids
    /// act as region ids: `ZoneRect` records for the id expand to Square
    /// cells at query time (rect interiors persist compactly, never as
    /// per-cell entities).
    pub fn cells_in_region(&self, region_id: &str) -> Vec<serde_json::Value> {
        let mut visited = std::collections::HashSet::new();
        let mut out = Vec::new();
        self.collect_region_cells(region_id, &mut visited, &mut out);
        out
    }

    /// Recursive helper for [`cells_in_region`](Self::cells_in_region).
    fn collect_region_cells(
        &self,
        region_id: &str,
        visited: &mut std::collections::HashSet<String>,
        out: &mut Vec<serde_json::Value>,
    ) {
        if !visited.insert(region_id.to_string()) {
            return;
        }
        for eid in self.get_entities_with_component("RegionAssignment") {
            let Some(val) = self.get_component(eid, "RegionAssignment") else {
                continue;
            };
            let Some(cell) = val.get("cell").cloned() else {
                continue;
            };
            let matches = match val.get("region_id") {
                Some(serde_json::Value::String(s)) => s == region_id,
                Some(serde_json::Value::Array(arr)) => {
                    arr.iter().any(|v| v.as_str() == Some(region_id))
                }
                _ => false,
            };
            if !matches {
                continue;
            }
            if let Some(nested) = cell
                .get("Region")
                .and_then(|r| r.get("id"))
                .and_then(|v| v.as_str())
            {
                self.collect_region_cells(nested, visited, out);
            } else {
                out.push(cell);
            }
        }
        let mut seen: std::collections::HashSet<String> =
            out.iter().map(|v| v.to_string()).collect();
        self.expand_zone_rects(region_id, out, &mut seen);
    }

    /// Returns all entity IDs assigned to the given region ID (supports multi-region).
    pub fn entities_in_region(&self, region_id: &str) -> Vec<u32> {
        self.get_entities_with_component("Region")
            .into_iter()
            .filter(|&eid| {
                self.get_component(eid, "Region")
                    .and_then(|val| val.get("id"))
                    .map(|id_val| match id_val {
                        serde_json::Value::String(s) => s == region_id,
                        serde_json::Value::Array(arr) => {
                            arr.iter().any(|v| v.as_str() == Some(region_id))
                        }
                        _ => false,
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns all entities assigned to regions of the given kind.
    ///
    /// Kind resolves through the region and zone record tables: entities
    /// whose `Region.kind` matches are included, as are entities whose
    /// `Region.id` names a zone carrying that kind (zone id acts as a region
    /// id, so zone members resolve through the same surface).
    pub fn entities_in_region_kind(&self, kind: &str) -> Vec<u32> {
        let zone_ids: std::collections::HashSet<String> = self
            .get_entities_with_component("Zone")
            .into_iter()
            .filter_map(|eid| {
                let val = self.get_component(eid, "Zone")?;
                if val.get("kind").and_then(|k| k.as_str()) != Some(kind) {
                    return None;
                }
                val.get("id")?.as_str().map(str::to_string)
            })
            .collect();
        self.get_entities_with_component("Region")
            .into_iter()
            .filter(|&eid| {
                let Some(val) = self.get_component(eid, "Region") else {
                    return false;
                };
                if val.get("kind").and_then(|k| k.as_str()) == Some(kind) {
                    return true;
                }
                match val.get("id") {
                    Some(serde_json::Value::String(s)) => zone_ids.contains(s),
                    Some(serde_json::Value::Array(arr)) => arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .any(|s| zone_ids.contains(s)),
                    _ => false,
                }
            })
            .collect()
    }

    /// Returns all cells assigned to regions of the given kind.
    ///
    /// Kind resolves through the region and zone record tables: every `Region`
    /// or `Zone` component whose `kind` matches contributes its `id`(s), and
    /// the result is the union of [`cells_in_region`](Self::cells_in_region)
    /// expansions for those ids (recursive `Region`-cell resolution with
    /// a cycle guard, plus `ZoneRect` query-time expansion). The
    /// `RegionAssignment` schema carries no `kind` field, so assignments are
    /// never filtered on one.
    pub fn cells_in_region_kind(&self, kind: &str) -> Vec<serde_json::Value> {
        let mut region_ids = Vec::new();
        for eid in self.get_entities_with_component("Region") {
            let Some(val) = self.get_component(eid, "Region") else {
                continue;
            };
            if val.get("kind").and_then(|k| k.as_str()) != Some(kind) {
                continue;
            }
            match val.get("id") {
                Some(serde_json::Value::String(s)) => region_ids.push(s.clone()),
                Some(serde_json::Value::Array(arr)) => {
                    for v in arr {
                        if let Some(s) = v.as_str() {
                            region_ids.push(s.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        for eid in self.get_entities_with_component("Zone") {
            let Some(val) = self.get_component(eid, "Zone") else {
                continue;
            };
            if val.get("kind").and_then(|k| k.as_str()) != Some(kind) {
                continue;
            }
            if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                region_ids.push(id.to_string());
            }
        }
        let mut visited = std::collections::HashSet::new();
        let mut out = Vec::new();
        for rid in region_ids {
            self.collect_region_cells(&rid, &mut visited, &mut out);
        }
        out
    }
}
