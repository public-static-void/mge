use super::PyWorld;
use crate::PyObject;
use engine_core::map::cell_key::CellKey;
use pyo3::prelude::*;
use serde_pyobject::to_pyobject;

/// API for the noise and detection system
pub trait NoiseApi {
    /// Set/update the NoiseEmitter component on an entity. Returns true on success.
    fn emit_noise(&self, entity: u32, intensity: f64, radius: u32) -> bool;
    /// Get the noise level at a cell. Returns 0.0 when no noise was propagated.
    fn get_noise_at(&self, x: i32, y: i32, z: i32) -> f64;
    /// Set/update the Hearing component on an entity. Returns true on success.
    fn set_hearing(&self, entity: u32, range: u32, sensitivity: f64) -> bool;
    /// Get the Hearing component data for an entity as a Python dict, or None.
    fn get_hearing(&self, py: Python<'_>, entity: u32) -> PyResult<Option<PyObject>>;
}

impl NoiseApi for PyWorld {
    fn emit_noise(&self, entity: u32, intensity: f64, radius: u32) -> bool {
        let mut world = self.inner.borrow_mut();
        let data = serde_json::json!({
            "intensity": intensity,
            "radius": radius,
            "active": true,
        });
        world.set_component(entity, "NoiseEmitter", data).is_ok()
    }

    fn get_noise_at(&self, x: i32, y: i32, z: i32) -> f64 {
        let world = self.inner.borrow();
        let cell = CellKey::Square { x, y, z };
        world.get_noise_at(&cell).unwrap_or(0.0)
    }

    fn set_hearing(&self, entity: u32, range: u32, sensitivity: f64) -> bool {
        let mut world = self.inner.borrow_mut();
        let data = serde_json::json!({
            "range": range,
            "sensitivity": sensitivity,
        });
        world.set_component(entity, "Hearing", data).is_ok()
    }

    fn get_hearing(&self, py: Python<'_>, entity: u32) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        match world.get_component(entity, "Hearing") {
            Some(data) => {
                let cloned = data.clone();
                let py_obj = to_pyobject(py, &cloned)?;
                Ok(Some(py_obj.into()))
            }
            None => Ok(None),
        }
    }
}
