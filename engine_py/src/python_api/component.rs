use super::PyWorld;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use serde_json::Value;
use serde_pyobject::{from_pyobject, to_pyobject};

/// Component API
pub trait ComponentApi {
    /// Set a component
    fn set_component(&self, entity_id: u32, name: String, value: Bound<'_, PyAny>) -> PyResult<()>;
    /// Get a component
    fn get_component(
        &self,
        py: Python<'_>,
        entity_id: u32,
        name: String,
    ) -> PyResult<Option<PyObject>>;
    /// Remove a component
    fn remove_component(&self, entity_id: u32, name: String) -> PyResult<()>;
    /// Get all entities with a component
    fn get_entities_with_component(&self, name: String) -> PyResult<Vec<u32>>;
    /// Get all entities with a list of components
    fn get_entities_with_components(&self, names: Vec<String>) -> Vec<u32>;
    /// List all components
    fn list_components(&self) -> Vec<String>;
    /// Get component schema
    fn get_component_schema(&self, name: String) -> PyResult<PyObject>;
}

impl ComponentApi for PyWorld {
    fn set_component(&self, entity_id: u32, name: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let json_value: Value = from_pyobject(value)?;
        world
            .set_component(entity_id, &name, json_value)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn get_component(
        &self,
        py: Python<'_>,
        entity_id: u32,
        name: String,
    ) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow_mut();
        if let Some(val) = world.get_component(entity_id, &name) {
            let cloned = val.clone();
            let py_obj = to_pyobject(py, &cloned)?;
            Ok(Some(py_obj.into()))
        } else {
            Ok(None)
        }
    }

    fn remove_component(&self, entity_id: u32, name: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        world
            .remove_component(entity_id, &name)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn get_entities_with_component(&self, name: String) -> PyResult<Vec<u32>> {
        let world = self.inner.borrow_mut();
        Ok(world.get_entities_with_component(&name))
    }

    fn get_entities_with_components(&self, names: Vec<String>) -> Vec<u32> {
        let world = self.inner.borrow_mut();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        world.get_entities_with_components(&name_refs)
    }

    fn list_components(&self) -> Vec<String> {
        let world = self.inner.borrow_mut();
        world.registry.lock().unwrap().all_component_names()
    }

    fn get_component_schema(&self, name: String) -> PyResult<PyObject> {
        let world = self.inner.borrow_mut();
        if let Some(schema) = world.registry.lock().unwrap().get_schema_by_name(&name) {
            let json = serde_json::to_value(&schema.schema)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            Python::with_gil(|py| Ok(to_pyobject(py, &json)?.into()))
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "Component schema not found",
            ))
        }
    }
}
