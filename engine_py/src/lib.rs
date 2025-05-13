use engine_core::ecs::event::{EventBus, EventReader};
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::scripting::world::World;
use pyo3::exceptions::PyIOError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};
use serde_json::Value;
use serde_pyobject::{from_pyobject, to_pyobject};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type EventBusMap = Mutex<HashMap<String, Arc<Mutex<EventBus<Value>>>>>;

/// Global registry of event buses, keyed by event type name.
static EVENT_BUSES: once_cell::sync::Lazy<EventBusMap> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

#[pyclass]
pub struct PyWorld {
    inner: Arc<Mutex<World>>,
}

#[pymethods]
impl PyWorld {
    #[new]
    #[pyo3(signature = (schema_dir=None))]
    fn new(schema_dir: Option<String>) -> PyResult<Self> {
        use std::path::PathBuf;

        let schema_path = match schema_dir {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from("engine/assets/schemas"),
        };

        let schemas = load_schemas_from_dir(&schema_path).map_err(|e| {
            PyValueError::new_err(format!(
                "Failed to load schemas from {:?}: {e}",
                schema_path
            ))
        })?;

        let mut registry = ComponentRegistry::new();
        for (_name, schema) in schemas {
            registry.register_external_schema(schema);
        }

        let world = World::new(Arc::new(registry));
        Ok(PyWorld {
            inner: Arc::new(Mutex::new(world)),
        })
    }

    fn spawn_entity(&self) -> u32 {
        let mut world = self.inner.lock().unwrap();
        world.spawn_entity()
    }

    fn despawn_entity(&self, entity_id: u32) {
        let mut world = self.inner.lock().unwrap();
        world.remove_entity(entity_id);
        world.entities.retain(|&e| e != entity_id);
    }

    fn set_component(&self, entity_id: u32, name: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        let mut world = self.inner.lock().unwrap();
        let json_value: Value = from_pyobject(value)?;
        world
            .set_component(entity_id, &name, json_value)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn get_component(
        &self,
        py: Python<'_>,
        entity_id: u32,
        name: String,
    ) -> PyResult<Option<PyObject>> {
        let world = self.inner.lock().unwrap();
        if let Some(val) = world.get_component(entity_id, &name) {
            let py_obj = to_pyobject(py, val)?;
            Ok(Some(py_obj.into()))
        } else {
            Ok(None)
        }
    }

    fn list_components(&self) -> Vec<String> {
        let world = self.inner.lock().unwrap();
        world.registry.all_component_names()
    }

    fn get_component_schema(&self, name: String) -> PyResult<PyObject> {
        let world = self.inner.lock().unwrap();
        if let Some(schema) = world.registry.get_schema_by_name(&name) {
            let json = serde_json::to_value(&schema.schema)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            Python::with_gil(|py| Ok(to_pyobject(py, &json)?.into()))
        } else {
            Err(PyValueError::new_err("Component schema not found"))
        }
    }

    fn remove_component(&self, entity_id: u32, name: String) {
        let mut world = self.inner.lock().unwrap();
        if let Some(comps) = world.components.get_mut(&name) {
            comps.remove(&entity_id);
        }
    }

    fn get_entities_with_component(&self, name: String) -> PyResult<Vec<u32>> {
        let world = self.inner.lock().unwrap();
        Ok(world.get_entities_with_component(&name))
    }

    fn get_entities_with_components(&self, names: Vec<String>) -> Vec<u32> {
        let world = self.inner.lock().unwrap();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        world.get_entities_with_components(&name_refs)
    }

    fn get_entities(&self) -> PyResult<Vec<u32>> {
        let world = self.inner.lock().unwrap();
        Ok(world.get_entities())
    }

    fn is_entity_alive(&self, entity_id: u32) -> bool {
        let world = self.inner.lock().unwrap();
        world.is_entity_alive(entity_id)
    }

    fn set_mode(&self, mode: String) {
        let mut world = self.inner.lock().unwrap();
        world.current_mode = mode;
    }

    fn get_mode(&self) -> String {
        let world = self.inner.lock().unwrap();
        world.current_mode.clone()
    }

    fn get_available_modes(&self) -> Vec<String> {
        let world = self.inner.lock().unwrap();
        world.registry.all_modes().into_iter().collect()
    }

    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32) {
        let mut world = self.inner.lock().unwrap();
        world.move_entity(entity_id, dx, dy);
    }

    fn move_all(&self, dx: f32, dy: f32) {
        let mut world = self.inner.lock().unwrap();
        world.move_all(dx, dy);
    }

    fn damage_entity(&self, entity_id: u32, amount: f32) {
        let mut world = self.inner.lock().unwrap();
        world.damage_entity(entity_id, amount);
    }

    fn damage_all(&self, amount: f32) {
        let mut world = self.inner.lock().unwrap();
        world.damage_all(amount);
    }

    fn tick(&self) {
        let mut world = self.inner.lock().unwrap();
        world.tick();
    }

    fn get_turn(&self) -> u32 {
        let world = self.inner.lock().unwrap();
        world.turn
    }

    fn process_deaths(&self) {
        let mut world = self.inner.lock().unwrap();
        world.process_deaths();
    }

    fn process_decay(&self) {
        let mut world = self.inner.lock().unwrap();
        world.process_decay();
    }

    fn count_entities_with_type(&self, type_str: String) -> usize {
        let world = self.inner.lock().unwrap();
        world.count_entities_with_type(&type_str)
    }

    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()> {
        let mut world = self.inner.lock().unwrap();
        world
            .modify_stockpile_resource(entity_id, &kind, delta)
            .map_err(PyValueError::new_err)
    }

    fn save_to_file(&self, path: String) -> PyResult<()> {
        let world = self.inner.lock().unwrap();
        world
            .save_to_file(std::path::Path::new(&path))
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn load_from_file(&mut self, path: String) -> PyResult<()> {
        let registry = {
            let world = self.inner.lock().unwrap();
            world.registry.clone()
        };
        let loaded = World::load_from_file(std::path::Path::new(&path), registry)
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        let mut world = self.inner.lock().unwrap();
        *world = loaded;
        Ok(())
    }

    /// Send an event of any type (event_type: str, payload: dict or primitive)
    fn send_event(&self, _py: Python, event_type: String, payload: String) -> PyResult<()> {
        let mut buses = EVENT_BUSES.lock().unwrap();
        let bus = buses
            .entry(event_type.clone())
            .or_insert_with(|| Arc::new(Mutex::new(EventBus::<Value>::default())))
            .clone();

        let json_payload: Value = serde_json::from_str(&payload)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        bus.lock().unwrap().send(json_payload);
        Ok(())
    }

    /// Poll all events of a type since last update (returns list of Python objects)
    fn poll_event(&self, py: Python, event_type: String) -> PyResult<Vec<PyObject>> {
        let buses = EVENT_BUSES.lock().unwrap();
        let bus = buses
            .get(&event_type)
            .ok_or_else(|| PyValueError::new_err(format!("No bus for event type {event_type}")))?;
        let mut reader = EventReader::default();
        let events: Vec<Value> = reader.read(&*bus.lock().unwrap()).cloned().collect();
        Ok(events
            .into_iter()
            .map(|e| to_pyobject(py, &e).unwrap().into())
            .collect())
    }

    /// Advance all event buses (should be called once per tick)
    fn update_event_buses(&self) {
        let buses = EVENT_BUSES.lock().unwrap();
        for bus in buses.values() {
            bus.lock().unwrap().update();
        }
    }
}

#[pymodule]
fn mge(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWorld>()?;
    Ok(())
}
