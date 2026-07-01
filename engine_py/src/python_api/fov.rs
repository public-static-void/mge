use super::PyWorld;
use crate::PyObject;
use engine_core::map::cell_key::CellKey;
use pyo3::prelude::*;
use serde_pyobject::to_pyobject;
use std::collections::HashMap;

/// API for the field-of-view system
pub trait FovApi {
    /// Get all visible cells for an entity as a list of {x, y, z} dicts
    fn get_visible_cells(&self, entity: u32) -> Vec<HashMap<String, i32>>;
    /// Check if a specific cell is visible to an entity
    fn is_visible(&self, entity: u32, x: i32, y: i32, z: i32) -> bool;
    /// Set/update the Sight component on an entity with the given range
    fn set_sight(&self, entity: u32, range: u32);
    /// Get the Sight component data for an entity as a Python dict, or None
    fn get_sight(&self, py: Python<'_>, entity: u32) -> PyResult<Option<PyObject>>;
    /// Switch the active FOV algorithm by registered name.
    fn set_fov_algorithm(&self, name: &str);
    // --- Fog-of-war API ---
    /// Check if a specific cell has been explored by an entity
    fn is_explored(&self, entity: u32, x: i32, y: i32, z: i32) -> bool;
    /// Get all explored cells for an entity as a list of {x, y, z} dicts
    fn get_explored_cells(&self, entity: u32) -> Vec<HashMap<String, i32>>;
    /// Reset (clear) fog-of-war for an entity
    fn reset_fog(&self, entity: u32);
    /// Get the visibility state of a cell for an entity (0=unexplored, 1=explored, 2=visible)
    fn get_visibility_state(&self, entity: u32, x: i32, y: i32, z: i32) -> u8;
}

impl FovApi for PyWorld {
    fn get_visible_cells(&self, entity: u32) -> Vec<HashMap<String, i32>> {
        let world = self.inner.borrow();
        match world.get_visible_cells(entity) {
            Some(cells) => cells
                .iter()
                .map(|cell| {
                    let mut map = HashMap::new();
                    match cell {
                        CellKey::Square { x, y, z } => {
                            map.insert("x".to_string(), *x);
                            map.insert("y".to_string(), *y);
                            map.insert("z".to_string(), *z);
                        }
                        CellKey::Hex { q, r, z } => {
                            map.insert("q".to_string(), *q);
                            map.insert("r".to_string(), *r);
                            map.insert("z".to_string(), *z);
                        }
                        CellKey::Province { id: _ } => {
                            // Province cells don't have x,y,z coordinates
                        }
                    }
                    map
                })
                .collect(),
            None => Vec::new(),
        }
    }

    fn is_visible(&self, entity: u32, x: i32, y: i32, z: i32) -> bool {
        let world = self.inner.borrow();
        let cell = CellKey::Square { x, y, z };
        world
            .get_visible_cells(entity)
            .map(|cells| cells.contains(&cell))
            .unwrap_or(false)
    }

    fn set_sight(&self, entity: u32, range: u32) {
        let mut world = self.inner.borrow_mut();
        let data = serde_json::json!({
            "range": range,
        });
        world.set_component(entity, "Sight", data).unwrap();
    }

    fn get_sight(&self, py: Python<'_>, entity: u32) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        match world.get_component(entity, "Sight") {
            Some(data) => {
                let cloned = data.clone();
                let py_obj = to_pyobject(py, &cloned)?;
                Ok(Some(py_obj.into()))
            }
            None => Ok(None),
        }
    }

    fn set_fov_algorithm(&self, name: &str) {
        let mut world = self.inner.borrow_mut();
        world
            .set_fov_algorithm_by_name(name)
            .expect("set_fov_algorithm failed");
    }

    fn is_explored(&self, entity: u32, x: i32, y: i32, z: i32) -> bool {
        let world = self.inner.borrow();
        let cell = CellKey::Square { x, y, z };
        world
            .get_explored_cells(entity)
            .map(|cells| cells.contains(&cell))
            .unwrap_or(false)
    }

    fn get_explored_cells(&self, entity: u32) -> Vec<HashMap<String, i32>> {
        let world = self.inner.borrow();
        match world.get_explored_cells(entity) {
            Some(cells) => cells
                .iter()
                .map(|cell| {
                    let mut map = HashMap::new();
                    match cell {
                        CellKey::Square { x, y, z } => {
                            map.insert("x".to_string(), *x);
                            map.insert("y".to_string(), *y);
                            map.insert("z".to_string(), *z);
                        }
                        CellKey::Hex { q, r, z } => {
                            map.insert("q".to_string(), *q);
                            map.insert("r".to_string(), *r);
                            map.insert("z".to_string(), *z);
                        }
                        CellKey::Province { id: _ } => {}
                    }
                    map
                })
                .collect(),
            None => Vec::new(),
        }
    }

    fn reset_fog(&self, entity: u32) {
        let mut world = self.inner.borrow_mut();
        world.reset_fog(entity);
    }

    fn get_visibility_state(&self, entity: u32, x: i32, y: i32, z: i32) -> u8 {
        let world = self.inner.borrow();
        let cell = CellKey::Square { x, y, z };
        world.get_visibility_state(entity, &cell)
    }
}
