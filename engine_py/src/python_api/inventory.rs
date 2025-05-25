use super::PyWorld;
use crate::python_api::component::ComponentApi;
use pyo3::prelude::*;
use pyo3::types::PyAny;

pub trait InventoryApi {
    fn get_inventory(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>>;
    fn set_inventory(&self, entity_id: u32, value: Bound<'_, PyAny>) -> PyResult<()>;
    fn add_item_to_inventory(&self, entity_id: u32, item_id: String) -> PyResult<()>;
    fn remove_item_from_inventory(
        &self,
        py: Python<'_>,
        entity_id: u32,
        index: usize,
    ) -> PyResult<()>;
}

impl InventoryApi for PyWorld {
    fn get_inventory(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>> {
        self.get_component(py, entity_id, "Inventory".to_string())
    }

    fn set_inventory(&self, entity_id: u32, value: Bound<'_, PyAny>) -> PyResult<()> {
        self.set_component(entity_id, "Inventory".to_string(), value)
    }

    fn add_item_to_inventory(&self, entity_id: u32, item_id: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let mut inv = if let Some(val) = world.get_component(entity_id, "Inventory") {
            val.clone()
        } else {
            serde_json::json!({})
        };
        let slots_opt = inv.get_mut("slots").and_then(|v| v.as_array_mut());
        let slots = if let Some(slots) = slots_opt {
            slots
        } else {
            inv["slots"] = serde_json::json!([]);
            inv.get_mut("slots").unwrap().as_array_mut().unwrap()
        };
        slots.push(serde_json::Value::String(item_id));
        world
            .set_component(entity_id, "Inventory", inv)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn remove_item_from_inventory(
        &self,
        _py: Python<'_>,
        entity_id: u32,
        index: usize,
    ) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let mut inv = if let Some(val) = world.get_component(entity_id, "Inventory") {
            val.clone()
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "No Inventory component found",
            ));
        };
        if let Some(slots) = inv.get_mut("slots").and_then(|v| v.as_array_mut()) {
            if index < slots.len() {
                slots.remove(index);
                world
                    .set_component(entity_id, "Inventory", inv)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
            } else {
                Err(pyo3::exceptions::PyValueError::new_err(
                    "Index out of bounds",
                ))
            }
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "No slots array in Inventory",
            ))
        }
    }
}
