use super::PyWorld;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use serde_json::Value;
use serde_pyobject::to_pyobject;

/// Equipment API
pub trait EquipmentApi {
    /// Get equipment data
    fn get_equipment(&self, py: Python<'_>, entity_id: u32) -> PyResult<PyObject>;
    /// Equip an item
    fn equip_item(&self, entity_id: u32, item_id: String, slot: String) -> PyResult<()>;
    /// Unequip an item
    fn unequip_item(&self, entity_id: u32, slot: String) -> PyResult<()>;
}

impl EquipmentApi for PyWorld {
    fn get_equipment(&self, py: Python<'_>, entity_id: u32) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        if let Some(val) = world.get_component(entity_id, "Equipment") {
            let any = to_pyobject(py, val)?;
            if let Ok(dict) = any.downcast::<PyDict>()
                && let Ok(Some(slots_any)) = dict.get_item("slots")
                && let Ok(slots) = slots_any.downcast::<PyDict>()
            {
                for (k, v) in slots.iter() {
                    if v.is_instance_of::<PyTuple>() && v.len().unwrap_or(1) == 0 {
                        slots.set_item(k, py.None())?;
                    }
                }
            }
            Ok(any.into())
        } else {
            Ok(PyDict::new(py).into())
        }
    }

    fn equip_item(&self, entity_id: u32, item_id: String, slot: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();

        // 1. Check inventory
        let inv = world
            .get_component(entity_id, "Inventory")
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Entity has no inventory"))?;
        let slots = inv.get("slots").and_then(|v| v.as_array()).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("No slots array in Inventory")
        })?;
        if !slots.iter().any(|v| v == &Value::String(item_id.clone())) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Item not in inventory",
            ));
        }

        // 2. Check item metadata
        let mut found = None;
        for item_eid in world.get_entities_with_component("Item") {
            if let Some(item_comp) = world.get_component(item_eid, "Item")
                && item_comp.get("id") == Some(&Value::String(item_id.clone()))
            {
                found = Some(item_comp);
                break;
            }
        }
        let item_meta =
            found.ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Item not found"))?;

        // 3. Check slot compatibility
        let valid_slot = item_meta
            .get("slot")
            .and_then(|v| v.as_str())
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Item missing slot info"))?;
        if valid_slot != slot {
            return Err(pyo3::exceptions::PyValueError::new_err("invalid slot"));
        }

        // 4. Get or create Equipment component with correct structure
        let mut equipment = if let Some(val) = world.get_component(entity_id, "Equipment") {
            val.clone()
        } else {
            let mut map = serde_json::Map::new();
            map.insert("slots".to_string(), serde_json::json!({}));
            Value::Object(map)
        };

        // 5. Ensure "slots" is an object
        let slots_obj = equipment
            .get_mut("slots")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Equipment slots must be an object")
            })?;

        // 6. Check if slot is already occupied
        if let Some(existing) = slots_obj.get(&slot)
            && !existing.is_null()
        {
            return Err(pyo3::exceptions::PyValueError::new_err("already equipped"));
        }

        // 7. Equip
        slots_obj.insert(slot.clone(), Value::String(item_id.clone()));
        world
            .set_component(entity_id, "Equipment", equipment)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn unequip_item(&self, entity_id: u32, slot: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let mut equipment = world
            .get_component(entity_id, "Equipment")
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("No Equipment component"))?
            .clone();
        let slots_obj = equipment
            .get_mut("slots")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Equipment slots must be an object")
            })?;
        slots_obj.insert(slot, Value::Null);
        world
            .set_component(entity_id, "Equipment", equipment)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}
