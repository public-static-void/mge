use super::PyWorld;
use pyo3::prelude::*;

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
