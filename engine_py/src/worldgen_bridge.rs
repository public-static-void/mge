use engine_core::worldgen::register_builtin_worldgen_plugins;
use engine_core::worldgen::{WorldgenPlugin, WorldgenRegistry};
use pyo3::Bound;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use serde_json::Value;
use serde_pyobject::{from_pyobject, to_pyobject};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static WORLDGEN_REGISTRY: Rc<RefCell<WorldgenRegistry>> = Rc::new(RefCell::new(WorldgenRegistry::new()));
}

#[pyfunction]
pub fn register_worldgen_plugin(py: Python, name: String, callback: Py<PyAny>) -> PyResult<()> {
    let cb = callback.clone_ref(py);
    WORLDGEN_REGISTRY.with(|registry| {
        registry.borrow_mut().register(WorldgenPlugin::Python {
            name,
            generate: Box::new(move |params| {
                Python::with_gil(|py| {
                    let arg = to_pyobject(py, params).unwrap();
                    let result = cb.call1(py, (arg,)).unwrap();
                    from_pyobject(result.bind(py).clone()).unwrap_or(serde_json::Value::Null)
                })
            }),
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
