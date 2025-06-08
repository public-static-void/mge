use engine_core::worldgen::{
    ScriptingWorldgenPlugin, WorldgenPlugin, WorldgenRegistry, register_builtin_worldgen_plugins,
};
use pyo3::prelude::*;
use pyo3::types::PyAny;
use serde_json::Value;
use serde_pyobject::{from_pyobject, to_pyobject};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static WORLDGEN_REGISTRY: Rc<RefCell<WorldgenRegistry>> = Rc::new(RefCell::new(WorldgenRegistry::new()));
}

struct PythonWorldgenPlugin {
    callback: Py<PyAny>,
}

impl ScriptingWorldgenPlugin for PythonWorldgenPlugin {
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
    WORLDGEN_REGISTRY.with(|registry| {
        registry.borrow_mut().register(WorldgenPlugin::Scripting {
            name,
            backend: "python".to_string(),
            opaque: Box::new(plugin),
        });
    });
    Ok(())
}

#[pyfunction]
pub fn list_worldgen_plugins() -> Vec<String> {
    WORLDGEN_REGISTRY.with(|registry| registry.borrow().list_names())
}

#[pyfunction]
pub fn invoke_worldgen_plugin<'py>(
    py: Python<'py>,
    name: String,
    params: Bound<'py, PyAny>,
) -> PyResult<PyObject> {
    let params: Value = serde_pyobject::from_pyobject(params)?;
    let result = WORLDGEN_REGISTRY
        .with(|registry| registry.borrow().invoke(&name, &params))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{:?}", e)))?;
    Ok(serde_pyobject::to_pyobject(py, &result)?.into())
}

// Call this once before running any worldgen plugin queries
#[pyfunction]
pub fn register_builtin_worldgen_plugins_py() {
    WORLDGEN_REGISTRY.with(|registry| {
        let mut reg = registry.borrow_mut();
        register_builtin_worldgen_plugins(&mut reg);
    });
}
