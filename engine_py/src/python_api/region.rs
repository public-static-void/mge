use super::PyWorld;
use pyo3::prelude::*;

pub trait RegionApi {
    fn get_entities_in_region(&self, region_id: String) -> Vec<u32>;
    fn get_entities_in_region_kind(&self, kind: String) -> Vec<u32>;
    fn get_cells_in_region(&self, py: Python, region_id: String) -> PyResult<PyObject>;
    fn get_cells_in_region_kind(&self, py: Python, kind: String) -> PyResult<PyObject>;
}

impl RegionApi for PyWorld {
    fn get_entities_in_region(&self, region_id: String) -> Vec<u32> {
        let world = self.inner.borrow();
        world.entities_in_region(&region_id)
    }

    fn get_entities_in_region_kind(&self, kind: String) -> Vec<u32> {
        let world = self.inner.borrow();
        world.entities_in_region_kind(&kind)
    }

    fn get_cells_in_region(&self, py: Python, region_id: String) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let cells = world.cells_in_region(&region_id);
        Ok(serde_pyobject::to_pyobject(py, &cells)?.into())
    }

    fn get_cells_in_region_kind(&self, py: Python, kind: String) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let cells = world.cells_in_region_kind(&kind);
        Ok(serde_pyobject::to_pyobject(py, &cells)?.into())
    }
}
