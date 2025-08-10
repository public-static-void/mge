use super::PyWorld;
use pyo3::prelude::*;

/// Economic API
pub trait EconomicApi {
    /// Modify the stockpile resource
    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()>;
}

impl EconomicApi for PyWorld {
    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        world
            .modify_stockpile_resource(entity_id, &kind, delta)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }
}

/// Get the resources of a stockpile
pub fn get_stockpile_resources(pyworld: &PyWorld, entity_id: u32) -> PyResult<Option<PyObject>> {
    let world = pyworld.inner.borrow();
    if let Some(stockpile) = world.get_component(entity_id, "Stockpile") {
        if let Some(resources) = stockpile.get("resources") {
            Python::with_gil(|py| Ok(Some(serde_pyobject::to_pyobject(py, resources)?.into())))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
