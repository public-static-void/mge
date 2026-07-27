use super::PyWorld;
use crate::PyObject;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::Value;

/// Unit template API
pub trait UnitTemplateApi {
    /// Load all .json template files from a directory.
    fn load_unit_templates(&self, dir: String) -> PyResult<()>;
    /// Register a single template from a JSON string.
    fn register_unit_template(&self, _name: String, template_json: String) -> PyResult<()>;
    /// Spawn an entity from a named template, with optional overrides dict.
    fn spawn_from_template(
        &self,
        template_name: String,
        overrides: Option<Bound<'_, PyDict>>,
    ) -> PyResult<u32>;
    /// Get a template definition by name.
    fn get_unit_template(&self, py: Python<'_>, name: String) -> PyResult<PyObject>;
    /// List all registered template names.
    fn list_unit_templates(&self) -> PyResult<Vec<String>>;
}

impl UnitTemplateApi for PyWorld {
    fn load_unit_templates(&self, dir: String) -> PyResult<()> {
        let path = std::path::Path::new(&dir);
        let mut world = self.inner.borrow_mut();
        world
            .template_registry
            .load_templates_from_dir(path)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn register_unit_template(&self, _name: String, template_json: String) -> PyResult<()> {
        let template: engine_core::ecs::template::UnitTemplate =
            serde_json::from_str(&template_json).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid template JSON: {e}"))
            })?;
        let mut world = self.inner.borrow_mut();
        world.template_registry.register_template(template);
        Ok(())
    }

    fn spawn_from_template(
        &self,
        template_name: String,
        overrides: Option<Bound<'_, PyDict>>,
    ) -> PyResult<u32> {
        let overrides_map = match overrides {
            Some(dict) => {
                let val: Value = serde_pyobject::from_pyobject(dict)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                match val {
                    Value::Object(map) => Some(map),
                    _ => {
                        return Err(pyo3::exceptions::PyValueError::new_err(
                            "Overrides must be a dict",
                        ));
                    }
                }
            }
            None => None,
        };
        let mut world = self.inner.borrow_mut();
        world
            .spawn_from_template(&template_name, overrides_map)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn get_unit_template(&self, py: Python<'_>, name: String) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        match world.template_registry.get_template(&name) {
            Some(tmpl) => {
                let val = serde_json::to_value(tmpl)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                let py_obj = serde_pyobject::to_pyobject(py, &val)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                Ok(py_obj.into())
            }
            None => Ok(py.None()),
        }
    }

    fn list_unit_templates(&self) -> PyResult<Vec<String>> {
        let world = self.inner.borrow();
        Ok(world.template_registry.list_templates())
    }
}
