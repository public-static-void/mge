use engine_core::ecs::world::World;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct SystemBridge {
    pub systems: RefCell<HashMap<String, Py<PyAny>>>,
}

impl SystemBridge {
    pub fn register_system(
        &self,
        py: Python,
        name: String,
        callback: Py<PyAny>,
        opts: Option<&Bound<'_, PyDict>>,
        world: Rc<RefCell<World>>,
    ) -> PyResult<()> {
        // Store callback for run_system lookup
        self.systems
            .borrow_mut()
            .insert(name.clone(), callback.clone_ref(py));

        // Extract dependencies from opts if provided
        let mut dependencies = Vec::new();
        if let Some(opts_dict) = opts
            && let Ok(Some(dep_item)) = opts_dict.get_item("dependencies")
            && let Ok(dep_list) = dep_item.cast::<PyList>()
        {
            for dep in dep_list.iter() {
                if let Ok(dep_str) = dep.extract::<String>() {
                    dependencies.push(dep_str);
                }
            }
        }

        // Register with engine core for dependency-ordered execution
        let cb = callback.clone_ref(py);
        world.borrow_mut().register_dynamic_system_with_deps(
            &name,
            dependencies,
            move |_world_rc, _dt| {
                let _ = Python::try_attach(|py| cb.call1(py, (0.0,)));
            },
        );

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
