use super::PyWorld;
use crate::PyObject;
use engine_core::ecs::EquipmentSet;
use pyo3::prelude::*;
use std::collections::HashMap;

/// Designer API trait for item definitions, equipment sets, and loadouts.
pub trait DesignerApi {
    fn load_item_definitions(&self, dir: String) -> PyResult<()>;
    fn register_item(&self, item_json: String) -> PyResult<()>;
    fn get_item_definition(&self, py: Python<'_>, id: String) -> PyResult<Option<PyObject>>;
    fn list_item_definitions(&self) -> PyResult<Vec<String>>;
    fn load_equipment_sets(&self, dir: String) -> PyResult<()>;
    fn define_equipment_set(&self, name: String, items: HashMap<String, String>) -> PyResult<()>;
    fn apply_loadout(&self, entity: u32, set_name: String) -> PyResult<u32>;
    fn get_loadout(&self, py: Python<'_>, entity: u32) -> PyResult<Option<PyObject>>;
    fn validate_equipment(&self, py: Python<'_>, entity: u32) -> PyResult<PyObject>;
}

impl DesignerApi for PyWorld {
    fn load_item_definitions(&self, dir: String) -> PyResult<()> {
        let path = std::path::Path::new(&dir);
        let mut world = self.inner.borrow_mut();
        world
            .item_registry
            .load_items_from_dir(path)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn register_item(&self, item_json: String) -> PyResult<()> {
        let definition: serde_json::Value = serde_json::from_str(&item_json).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid item JSON: {e}"))
        })?;
        let mut world = self.inner.borrow_mut();
        world
            .item_registry
            .register_item(definition)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn get_item_definition(&self, py: Python<'_>, id: String) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        match world.item_registry.get_item(&id) {
            Some(def) => {
                let val = serde_json::to_value(def)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                let py_obj = serde_pyobject::to_pyobject(py, &val)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                Ok(Some(py_obj.into()))
            }
            None => Ok(None),
        }
    }

    fn list_item_definitions(&self) -> PyResult<Vec<String>> {
        let world = self.inner.borrow();
        Ok(world.item_registry.list_items())
    }

    fn load_equipment_sets(&self, dir: String) -> PyResult<()> {
        let path = std::path::Path::new(&dir);
        let mut world = self.inner.borrow_mut();
        world
            .equipment_set_registry
            .load_sets_from_dir(path)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn define_equipment_set(&self, name: String, items: HashMap<String, String>) -> PyResult<()> {
        let set = EquipmentSet {
            name: name.clone(),
            version: "1.0.0".to_string(),
            description: String::new(),
            items,
        };
        let mut world = self.inner.borrow_mut();
        world.equipment_set_registry.register_set(set);
        Ok(())
    }

    fn apply_loadout(&self, entity: u32, set_name: String) -> PyResult<u32> {
        let mut world = self.inner.borrow_mut();
        world
            .apply_loadout(entity, &set_name)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn get_loadout(&self, py: Python<'_>, entity: u32) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();

        let equipment = match world.get_component(entity, "Equipment") {
            Some(e) => e,
            None => return Ok(None),
        };

        let entity_slots = match equipment.get("slots").and_then(|v| v.as_object()) {
            Some(s) => s,
            None => return Ok(None),
        };

        let entity_items: HashMap<&str, &str> = entity_slots
            .iter()
            .filter_map(|(slot, item_id)| item_id.as_str().map(|id| (slot.as_str(), id)))
            .collect();

        let set_names = world.equipment_set_registry.list_sets();
        for set_name in &set_names {
            if let Some(set) = world.equipment_set_registry.get_set(set_name)
                && set.items.len() == entity_items.len()
                && set.items.iter().all(|(slot, item_id)| {
                    entity_items.get(slot.as_str()) == Some(&item_id.as_str())
                })
            {
                let result = serde_json::json!({
                    "name": set.name,
                    "items": set.items,
                });
                let py_obj = serde_pyobject::to_pyobject(py, &result)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                return Ok(Some(py_obj.into()));
            }
        }

        Ok(None)
    }

    fn validate_equipment(&self, py: Python<'_>, entity: u32) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let issues = world.validate_equipment(entity);

        let result = serde_json::json!({
            "issues": issues.iter().map(|i| {
                serde_json::json!({
                    "slot": i.slot,
                    "item_id": i.item_id,
                    "reason": i.reason,
                })
            }).collect::<Vec<_>>(),
        });

        serde_pyobject::to_pyobject(py, &result)
            .map(|b| b.into())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}
