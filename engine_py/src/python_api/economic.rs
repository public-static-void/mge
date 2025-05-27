use super::PyWorld;
use pyo3::prelude::*;
use serde_pyobject::to_pyobject;

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
