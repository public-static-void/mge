use super::World;
use crate::ecs::components::position::{Position, PositionComponent};

impl World {
    pub fn spawn_entity(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(id);
        id
    }

    pub fn despawn_entity(&mut self, entity: u32) {
        for (_comp_name, comps) in self.components.iter_mut() {
            let _existed = comps.remove(&entity).is_some();
        }
        self.entities.retain(|&id| id != entity);
    }

    pub fn entity_exists(&self, entity: u32) -> bool {
        let in_entities = self.entities.contains(&entity);
        let in_any_component = self
            .components
            .values()
            .any(|comp_map| comp_map.contains_key(&entity));
        in_entities || in_any_component
    }

    pub fn get_entities(&self) -> Vec<u32> {
        self.entities.clone()
    }

    pub fn get_entities_with_component(&self, name: &str) -> Vec<u32> {
        if !self.is_component_allowed_in_mode(name, &self.current_mode) {
            return vec![];
        }
        self.components
            .get(name)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn has_component(&self, entity: u32, name: &str) -> bool {
        self.components
            .get(name)
            .is_some_and(|m| m.contains_key(&entity))
    }

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

    pub fn damage_entity(&mut self, entity: u32, amount: f32) {
        if let Some(healths) = self.components.get_mut("Health")
            && let Some(value) = healths.get_mut(&entity)
            && let Some(obj) = value.as_object_mut()
            && let Some(current) = obj.get_mut("current")
            && let Some(cur_val) = current.as_f64()
        {
            *current = serde_json::json!((cur_val - amount as f64).max(0.0));
        }
    }

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
    pub fn entities_in_cell(&self, cell: &crate::map::CellKey) -> Vec<u32> {
        self.entities
            .iter()
            .copied()
            .filter(|&eid| {
                self.get_component(eid, "Position")
                    .and_then(|val| {
                        val.get("pos").and_then(|p| {
                            if let Some(obj) = p.as_object()
                                && let Some(sq) = obj.get("Square")
                            {
                                let x = sq.get("x")?.as_i64()? as i32;
                                let y = sq.get("y")?.as_i64()? as i32;
                                let z = sq.get("z")?.as_i64()? as i32;
                                if let crate::map::CellKey::Square {
                                    x: cx,
                                    y: cy,
                                    z: cz,
                                } = cell
                                {
                                    return Some(*cx == x && *cy == y && *cz == z);
                                }
                            }
                            None
                        })
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns all entity IDs in the given z-level (for SquareGridMap).
    pub fn entities_in_zlevel(&self, z: i32) -> Vec<u32> {
        self.entities
            .iter()
            .copied()
            .filter(|&eid| {
                self.get_component(eid, "Position")
                    .and_then(|val| {
                        val.get("pos").and_then(|p| {
                            if let Some(obj) = p.as_object()
                                && let Some(sq) = obj.get("Square")
                            {
                                let zval = sq.get("z")?.as_i64()? as i32;
                                return Some(zval == z);
                            }
                            None
                        })
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns all cells (as serde_json::Value) assigned to the given region_id.
    pub fn cells_in_region(&self, region_id: &str) -> Vec<serde_json::Value> {
        self.get_entities_with_component("RegionAssignment")
            .into_iter()
            .filter_map(|eid| {
                self.get_component(eid, "RegionAssignment").and_then(|val| {
                    let cell = val.get("cell").cloned()?;
                    let rid = val.get("region_id");
                    match rid {
                        Some(serde_json::Value::String(s)) => {
                            if s == region_id {
                                Some(cell)
                            } else {
                                None
                            }
                        }
                        Some(serde_json::Value::Array(arr)) => {
                            if arr.iter().any(|v| v.as_str() == Some(region_id)) {
                                Some(cell)
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                })
            })
            .collect()
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
    pub fn entities_in_region_kind(&self, kind: &str) -> Vec<u32> {
        self.get_entities_with_component("Region")
            .into_iter()
            .filter(|&eid| {
                self.get_component(eid, "Region")
                    .and_then(|val| val.get("kind"))
                    .and_then(|k| k.as_str())
                    .map(|k| k == kind)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns all cells assigned to regions of the given kind.
    pub fn cells_in_region_kind(&self, kind: &str) -> Vec<serde_json::Value> {
        self.get_entities_with_component("RegionAssignment")
            .into_iter()
            .filter_map(|eid| {
                self.get_component(eid, "RegionAssignment").and_then(|val| {
                    let k = val.get("kind").and_then(|v| v.as_str());
                    let cell = val.get("cell").cloned();
                    if k == Some(kind) { cell } else { None }
                })
            })
            .collect()
    }
}
