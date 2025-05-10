use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::scripting::world::World;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use serde_pyobject::{from_pyobject, to_pyobject};
use std::sync::{Arc, Mutex};

#[pyclass]
pub struct PyWorld {
    inner: Arc<Mutex<World>>,
}

#[pymethods]
impl PyWorld {
    #[new]
    #[pyo3(signature = (schema_dir=None))]
    fn new(schema_dir: Option<String>) -> PyResult<Self> {
        use std::path::PathBuf;

        let schema_path = match schema_dir {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from("engine/assets/schemas"),
        };

        let schemas = load_schemas_from_dir(&schema_path).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Failed to load schemas from {:?}: {e}",
                schema_path
            ))
        })?;

        let mut registry = ComponentRegistry::new();
        for (_name, schema) in schemas {
            registry.register_external_schema(schema);
        }

        let world = World::new(Arc::new(registry));
        Ok(PyWorld {
            inner: Arc::new(Mutex::new(world)),
        })
    }

    fn spawn(&self) -> u32 {
        let mut world = self.inner.lock().unwrap();
        world.spawn()
    }

    fn set_component(
        &self,
        entity: u32,
        name: String,
        value: Bound<'_, pyo3::types::PyAny>,
    ) -> PyResult<()> {
        let mut world = self.inner.lock().unwrap();
        let json_value: serde_json::Value = from_pyobject(value)?;
        world
            .set_component(entity, &name, json_value)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn get_component(
        &self,
        py: Python<'_>,
        entity: u32,
        name: String,
    ) -> PyResult<Option<PyObject>> {
        let world = self.inner.lock().unwrap();
        if let Some(val) = world.get_component(entity, &name) {
            let py_obj = to_pyobject(py, val)?;
            Ok(Some(py_obj.into()))
        } else {
            Ok(None)
        }
    }
}

#[pymodule]
fn mge(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWorld>()?;
    Ok(())
}
