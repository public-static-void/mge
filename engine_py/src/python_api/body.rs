use super::PyWorld;
use crate::python_api::component::ComponentApi;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use serde_json::Value;
use serde_pyobject::from_pyobject;

pub trait BodyApi {
    fn get_body(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>>;
    fn set_body(&self, entity_id: u32, value: Bound<'_, PyAny>) -> PyResult<()>;
    fn add_body_part(&self, entity_id: u32, part: Bound<'_, PyAny>) -> PyResult<()>;
    fn remove_body_part(&self, entity_id: u32, part_name: String) -> PyResult<()>;
    fn get_body_part(
        &self,
        py: Python<'_>,
        entity_id: u32,
        part_name: String,
    ) -> PyResult<Option<PyObject>>;
}

impl BodyApi for PyWorld {
    fn get_body(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>> {
        self.get_component(py, entity_id, "Body".to_string())
    }

    fn set_body(&self, entity_id: u32, value: Bound<'_, PyAny>) -> PyResult<()> {
        self.set_component(entity_id, "Body".to_string(), value)
    }

    fn add_body_part(&self, entity_id: u32, part: Bound<'_, PyAny>) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let mut body = if let Some(val) = world.get_component(entity_id, "Body") {
            val.clone()
        } else {
            serde_json::json!({})
        };
        let parts = body.get_mut("parts").and_then(|v| v.as_array_mut());
        let parts = if let Some(parts) = parts {
            parts
        } else {
            body["parts"] = serde_json::json!([]);
            body.get_mut("parts").unwrap().as_array_mut().unwrap()
        };
        let part_json: Value = from_pyobject(part)?;
        parts.push(part_json);
        world
            .set_component(entity_id, "Body", body)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn remove_body_part(&self, entity_id: u32, part_name: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let mut body = if let Some(val) = world.get_component(entity_id, "Body") {
            val.clone()
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "No Body component found",
            ));
        };

        fn remove_part_recursive(parts: &mut Vec<Value>, name: &str) -> bool {
            let mut i = 0;
            while i < parts.len() {
                let part = &mut parts[i];
                if part.get("name").and_then(|n| n.as_str()) == Some(name) {
                    parts.remove(i);
                    return true;
                }
                if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut()) {
                    if remove_part_recursive(children, name) {
                        return true;
                    }
                }
                i += 1;
            }
            false
        }

        if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
            if remove_part_recursive(parts, &part_name) {
                world
                    .set_component(entity_id, "Body", body)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
            } else {
                Err(pyo3::exceptions::PyValueError::new_err(
                    "Body part not found",
                ))
            }
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "No parts array in Body",
            ))
        }
    }

    fn get_body_part(
        &self,
        py: Python<'_>,
        entity_id: u32,
        part_name: String,
    ) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        if let Some(body) = world.get_component(entity_id, "Body") {
            fn find_part<'a>(parts: &'a [Value], name: &str) -> Option<&'a Value> {
                for part in parts {
                    if part.get("name").and_then(|n| n.as_str()) == Some(name) {
                        return Some(part);
                    }
                    if let Some(children) = part.get("children").and_then(|v| v.as_array()) {
                        if let Some(found) = find_part(children, name) {
                            return Some(found);
                        }
                    }
                }
                None
            }
            if let Some(parts) = body.get("parts").and_then(|v| v.as_array()) {
                if let Some(part) = find_part(parts, &part_name) {
                    let py_obj = serde_pyobject::to_pyobject(py, part)?;
                    return Ok(Some(py_obj.into()));
                }
            }
        }
        Ok(None)
    }
}
