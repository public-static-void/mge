use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct SystemBridge {
    pub systems: RefCell<HashMap<String, Py<PyAny>>>,
}

impl SystemBridge {
    pub fn register_system(&self, py: Python, name: String, callback: Py<PyAny>) -> PyResult<()> {
        self.systems
            .borrow_mut()
            .insert(name, callback.clone_ref(py));
        Ok(())
    }

    pub fn run_system(&self, py: Python, name: String) -> PyResult<()> {
        if let Some(cb) = self.systems.borrow().get(&name) {
            cb.call1(py, (0.0,))?;
            Ok(())
        } else {
            Err(PyValueError::new_err("System not found"))
        }
    }
}
