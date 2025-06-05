use super::PyWorld;
use pyo3::prelude::*;
use serde_pyobject::to_pyobject;

pub trait EconomicApi {
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

#[pymethods]
impl PyWorld {
    pub fn get_stockpile_resources(&self, entity_id: u32) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
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

    pub fn get_production_job(&self, entity_id: u32) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        if let Some(job) = world.get_component(entity_id, "ProductionJob") {
            Python::with_gil(|py| Ok(Some(serde_pyobject::to_pyobject(py, job)?.into())))
        } else {
            Ok(None)
        }
    }
}
