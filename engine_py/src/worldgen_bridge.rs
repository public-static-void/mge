use engine_core::worldgen::{WorldgenPlugin, WorldgenRegistry};
use pyo3::prelude::*;
use pyo3::types::PyAny;
use serde_json::Value;
use serde_pyobject::{from_pyobject, to_pyobject};

pub struct WorldgenBridge {
    pub worldgen_registry: std::cell::RefCell<WorldgenRegistry>,
}

impl WorldgenBridge {
    pub fn register_worldgen(&self, py: Python, name: String, callback: Py<PyAny>) -> PyResult<()> {
        let cb = callback.clone_ref(py);
        self.worldgen_registry
            .borrow_mut()
            .register(WorldgenPlugin::Python {
                name,
                generate: Box::new(move |params| {
                    Python::with_gil(|py| {
                        let arg = to_pyobject(py, params).unwrap();
                        let result = cb.call1(py, (arg,)).unwrap();
                        from_pyobject(result.bind(py).clone()).unwrap_or(serde_json::Value::Null)
                    })
                }),
            });
        Ok(())
    }

    pub fn list_worldgen(&self) -> Vec<String> {
        self.worldgen_registry.borrow().list_names()
    }

    pub fn invoke_worldgen<'py>(
        &self,
        py: Python<'py>,
        name: String,
        params: Bound<'py, PyAny>,
    ) -> PyResult<PyObject> {
        let params: Value = serde_pyobject::from_pyobject(params)?;
        let result = self
            .worldgen_registry
            .borrow()
            .invoke(&name, &params)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))?;
        Ok(serde_pyobject::to_pyobject(py, &result)?.into())
    }
}
