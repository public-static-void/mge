use super::World;

impl World {
    pub fn spawn_entity(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(id);
        id
    }

    pub fn despawn_entity(&mut self, entity: u32) {
        for comps in self.components.values_mut() {
            comps.remove(&entity);
        }
        self.entities.retain(|&id| id != entity);
    }

    pub fn get_entities(&self) -> Vec<u32> {
        self.entities.clone()
    }

    pub fn get_entities_with_component(&self, name: &str) -> Vec<u32> {
        self.components
            .get(name)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_entities_with_components(&self, names: &[&str]) -> Vec<u32> {
        if names.is_empty() {
            return self.entities.clone();
        }
        let mut sets: Vec<std::collections::HashSet<u32>> = names
            .iter()
            .filter_map(|name| self.components.get(*name))
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

    /// Returns all entity IDs in the given cell.
    pub fn entities_in_cell(&self, cell: &crate::map::CellKey) -> Vec<u32> {
        self.entities
            .iter()
            .copied()
            .filter(|&eid| {
                self.get_component(eid, "PositionComponent")
                    .and_then(|val| {
                        val.get("pos").and_then(|p| {
                            if let Some(obj) = p.as_object() {
                                if let Some(sq) = obj.get("Square") {
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
                self.get_component(eid, "PositionComponent")
                    .and_then(|val| {
                        val.get("pos").and_then(|p| {
                            if let Some(obj) = p.as_object() {
                                if let Some(sq) = obj.get("Square") {
                                    let zval = sq.get("z")?.as_i64()? as i32;
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

    /// Returns all entity IDs assigned to the given region ID.
    pub fn entities_in_region(&self, region_id: &str) -> Vec<u32> {
        self.get_entities_with_component("Region")
            .into_iter()
            .filter(|&eid| {
                self.get_component(eid, "Region")
                    .and_then(|val| val.get("id"))
                    .and_then(|id| id.as_str())
                    .map(|id| id == region_id)
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
                    let rid = val.get("region_id").and_then(|id| id.as_str());
                    let cell = val.get("cell").cloned();
                    if rid == Some(region_id) { cell } else { None }
                })
            })
            .collect()
    }
}
