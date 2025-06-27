use engine_core::worldgen::{
    GLOBAL_WORLDGEN_REGISTRY, ThreadSafeScriptingWorldgenPlugin, ThreadSafeWorldgenPlugin,
};
use pyo3::prelude::*;
use pyo3::types::PyAny;
use serde_json::Value;
use serde_pyobject::{from_pyobject, to_pyobject};

struct PythonWorldgenPlugin {
    callback: Py<PyAny>,
}

impl Clone for PythonWorldgenPlugin {
    fn clone(&self) -> Self {
        // SAFETY: clone_ref requires the GIL.
        Python::with_gil(|py| PythonWorldgenPlugin {
            callback: self.callback.clone_ref(py),
        })
    }
}

impl ThreadSafeScriptingWorldgenPlugin for PythonWorldgenPlugin {
    fn invoke(&self, params: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        Python::with_gil(|py| {
            let arg = to_pyobject(py, params)?;
            let result = self.callback.call1(py, (arg,))?;
            from_pyobject(result.bind(py).clone()).map_err(|e| Box::new(e) as _)
        })
    }
    fn backend(&self) -> &str {
        "python"
    }
}

#[pyfunction]
pub fn register_worldgen_plugin(py: Python, name: String, callback: Py<PyAny>) -> PyResult<()> {
    let plugin = PythonWorldgenPlugin {
        callback: callback.clone_ref(py),
    };
    let mut registry = GLOBAL_WORLDGEN_REGISTRY.lock().unwrap();
    registry.register(ThreadSafeWorldgenPlugin::ThreadSafeScripting {
        name,
        backend: "python".to_string(),
        opaque: Box::new(plugin),
    });
    Ok(())
}

#[pyfunction]
pub fn list_worldgen_plugins() -> Vec<String> {
    let registry = GLOBAL_WORLDGEN_REGISTRY.lock().unwrap();
    registry.list_names()
}

#[pyfunction]
pub fn invoke_worldgen_plugin<'py>(
    py: Python<'py>,
    name: String,
    params: Bound<'py, PyAny>,
) -> PyResult<PyObject> {
    let params: Value = serde_pyobject::from_pyobject(params)?;
    let registry = GLOBAL_WORLDGEN_REGISTRY.lock().unwrap();
    let result = registry
        .invoke(&name, &params)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{e:?}")))?;
    Ok(serde_pyobject::to_pyobject(py, &result)?.into())
}

#[pyfunction]
pub fn register_worldgen_validator(py: Python, callback: Py<PyAny>) -> PyResult<()> {
    let cb = callback.clone_ref(py);
    let mut registry = GLOBAL_WORLDGEN_REGISTRY.lock().unwrap();
    registry.register_validator(move |map| {
        Python::with_gil(|py| {
            let arg = to_pyobject(py, map).map_err(|e| e.to_string())?;
            let result = cb.call1(py, (arg,)).map_err(|e| e.to_string())?;
            if result.is_truthy(py).map_err(|e| e.to_string())? {
                Ok(())
            } else {
                Err("Validator failed".to_string())
            }
        })
    });
    Ok(())
}

#[pyfunction]
pub fn register_worldgen_postprocessor(py: Python, callback: Py<PyAny>) -> PyResult<()> {
    let cb = callback.clone_ref(py);
    let mut registry = GLOBAL_WORLDGEN_REGISTRY.lock().unwrap();
    registry.register_postprocessor(move |map| {
        Python::with_gil(|py| {
            let arg = to_pyobject(py, map).expect("Failed to convert map to PyObject");
            let result: PyObject = cb
                .call1(py, (arg.clone(),))
                .expect("Postprocessor call failed");
            // If the Python function returned a dict, use it to update the map
            if let Ok(dict) = result.extract::<pyo3::Bound<'_, pyo3::types::PyDict>>(py) {
                let new_map: Value =
                    from_pyobject(dict).expect("Failed to convert PyDict to Value");
                *map = new_map;
            } else if let Ok(bound_any) = arg.extract::<pyo3::Bound<'_, pyo3::types::PyAny>>() {
                if let Ok(dict) = bound_any.extract::<pyo3::Bound<'_, pyo3::types::PyDict>>() {
                    // If the user mutated the dict in place, update map from arg
                    let new_map: Value =
                        from_pyobject(dict).expect("Failed to convert PyDict to Value");
                    *map = new_map;
                }
            }
        });
    });
    Ok(())
}
