use super::PyWorld;
use pyo3::prelude::*;

/// Methods for accessing entities in regions
pub trait RegionApi {
    /// Get all entities in a region
    fn get_entities_in_region(&self, region_id: String) -> Vec<u32>;
    /// Get all entities of a kind in a region
    fn get_entities_in_region_kind(&self, kind: String) -> Vec<u32>;
    /// Get all cells in a region
    fn get_cells_in_region(&self, py: Python, region_id: String) -> PyResult<PyObject>;
    /// Get all cells of a kind in a region
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
