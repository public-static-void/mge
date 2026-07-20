//! Material system Python helpers: lookup, attach, query.

use crate::PyObject;
use crate::python_api::world::PyWorld;
use engine_core::material;
use pyo3::prelude::*;

/// Material API methods implemented as a trait for PyWorld.
pub trait MaterialApi {
    /// Return material properties for the given name, or None.
    fn get_material_properties(&self, py: Python<'_>, name: String) -> PyResult<Option<PyObject>>;

    /// Attach a Material component to an entity. Raises ValueError on unknown name.
    fn set_entity_material(&self, entity_id: u32, material_name: String) -> PyResult<()>;

    /// Return the Material component for an entity, or None.
    fn get_entity_material(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>>;

    /// Return all registered material names.
    fn get_material_names(&self) -> Vec<String>;
}

impl MaterialApi for PyWorld {
    fn get_material_properties(&self, py: Python<'_>, name: String) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        if world.material_definitions.contains_key(&name) {
            let val = material::get_material_properties(&world, &name);
            let obj = serde_pyobject::to_pyobject(py, &val)
                .map(|b| b.into())
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            Ok(Some(obj))
        } else {
            Ok(None)
        }
    }

    fn set_entity_material(&self, entity_id: u32, material_name: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        material::set_entity_material(&mut world, entity_id, &material_name)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn get_entity_material(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        match material::get_entity_material(&world, entity_id) {
            Some(val) => {
                let obj = serde_pyobject::to_pyobject(py, &val)
                    .map(|b| b.into())
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                Ok(Some(obj))
            }
            None => Ok(None),
        }
    }

    fn get_material_names(&self) -> Vec<String> {
        let world = self.inner.borrow();
        material::get_material_names(&world)
    }
}
