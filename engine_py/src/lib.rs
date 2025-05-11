use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::scripting::world::World;
use pyo3::PyObject;
use pyo3::Python;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use serde_pyobject::{from_pyobject, to_pyobject};
use std::sync::{Arc, Mutex};

#[pyclass]
pub struct PyWorld {
    inner: Arc<Mutex<World>>,
}

#[pymethods]
impl PyWorld {
    /// Create a new world, optionally loading schemas from the given directory.
    #[new]
    #[pyo3(signature = (schema_dir=None))]
    fn new(schema_dir: Option<String>) -> PyResult<Self> {
        use std::path::PathBuf;

        let schema_path = match schema_dir {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from("engine/assets/schemas"),
        };

        let schemas = load_schemas_from_dir(&schema_path).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
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

    /// Spawn a new entity and return its ID.
    fn spawn_entity(&self) -> u32 {
        let mut world = self.inner.lock().unwrap();
        world.spawn_entity()
    }

    /// Remove an entity and all its components.
    fn despawn_entity(&self, entity: u32) {
        let mut world = self.inner.lock().unwrap();
        world.remove_entity(entity);
        // Optionally, also remove from world.entities if not already done
        world.entities.retain(|&e| e != entity);
    }

    /// Set a component on an entity.
    ///
    /// Args:
    ///     entity: Entity ID.
    ///     name: Component name.
    ///     value: Component data as a Python dict.
    fn set_component(
        &self,
        entity: u32,
        name: String,
        value: Bound<'_, pyo3::types::PyAny>,
    ) -> PyResult<()> {
        let mut world = self.inner.lock().unwrap();
        let json_value: serde_json::Value = from_pyobject(value)?;
        world
            .set_component(entity, &name, json_value)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Get a component's data from an entity.
    ///
    /// Returns the component data as a Python dict, or None if not present.
    fn get_component(
        &self,
        py: Python<'_>,
        entity: u32,
        name: String,
    ) -> PyResult<Option<PyObject>> {
        let world = self.inner.lock().unwrap();
        if let Some(val) = world.get_component(entity, &name) {
            let py_obj = to_pyobject(py, val)?;
            Ok(Some(py_obj.into()))
        } else {
            Ok(None)
        }
    }

    /// List all registered component names.
    fn list_components(&self) -> Vec<String> {
        let world = self.inner.lock().unwrap();
        world.registry.all_component_names()
    }

    /// Get the JSON schema of a component as a Python dict.
    ///
    /// Raises ValueError if the component schema is not found.
    fn get_component_schema(&self, name: String) -> PyResult<PyObject> {
        let world = self.inner.lock().unwrap();
        if let Some(schema) = world.registry.get_schema_by_name(&name) {
            let json = serde_json::to_value(&schema.schema)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            Python::with_gil(|py| Ok(serde_pyobject::to_pyobject(py, &json)?.into()))
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "Component schema not found",
            ))
        }
    }

    /// Remove a component from an entity.
    fn remove_component(&self, entity: u32, name: String) {
        let mut world = self.inner.lock().unwrap();
        if let Some(comps) = world.components.get_mut(&name) {
            comps.remove(&entity);
        }
    }

    /// Get all entity IDs that have a given component.
    fn get_entities_with_component(&self, name: String) -> PyResult<Vec<u32>> {
        let world = self.inner.lock().unwrap();
        Ok(world.get_entities_with_component(&name))
    }

    /// Get all entity IDs that have given components.
    fn get_entities_with_components(&self, names: Vec<String>) -> Vec<u32> {
        let world = self.inner.lock().unwrap();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        world.get_entities_with_components(&name_refs)
    }

    /// Get a list of all entity IDs in the world.
    fn get_entities(&self) -> PyResult<Vec<u32>> {
        let world = self.inner.lock().unwrap();
        Ok(world.get_entities())
    }

    /// Return True if the entity is considered alive (Health > 0).
    fn is_entity_alive(&self, entity: u32) -> bool {
        let world = self.inner.lock().unwrap();
        world.is_entity_alive(entity)
    }

    /// Set the current game mode.
    fn set_mode(&self, mode: String) {
        let mut world = self.inner.lock().unwrap();
        world.current_mode = mode;
    }

    /// Get the current game mode.
    fn get_mode(&self) -> String {
        let world = self.inner.lock().unwrap();
        world.current_mode.clone()
    }

    /// List all available game modes.
    fn get_available_modes(&self) -> Vec<String> {
        let world = self.inner.lock().unwrap();
        world.registry.all_modes().into_iter().collect()
    }

    /// Move an entity by (dx, dy).
    fn move_entity(&self, entity: u32, dx: f32, dy: f32) {
        let mut world = self.inner.lock().unwrap();
        world.move_entity(entity, dx, dy);
    }

    /// Move all entities with a Position component by (dx, dy).
    fn move_all(&self, dx: f32, dy: f32) {
        let mut world = self.inner.lock().unwrap();
        world.move_all(dx, dy);
    }

    /// Damage an entity (reduces its Health).
    fn damage_entity(&self, entity: u32, amount: f32) {
        let mut world = self.inner.lock().unwrap();
        world.damage_entity(entity, amount);
    }

    /// Damage all entities with a Health component.
    fn damage_all(&self, amount: f32) {
        let mut world = self.inner.lock().unwrap();
        world.damage_all(amount);
    }

    /// Advance the game simulation by one tick.
    fn tick(&self) {
        let mut world = self.inner.lock().unwrap();
        world.tick();
    }

    /// Get the current turn number.
    fn get_turn(&self) -> u32 {
        let world = self.inner.lock().unwrap();
        world.turn
    }

    /// Process deaths for entities with zero or less Health.
    fn process_deaths(&self) {
        let mut world = self.inner.lock().unwrap();
        world.process_deaths();
    }

    /// Process decay timers and remove decayed entities.
    fn process_decay(&self) {
        let mut world = self.inner.lock().unwrap();
        world.process_decay();
    }

    /// Count entities with Type.kind equal to the given string.
    fn count_entities_with_type(&self, type_str: String) -> usize {
        let world = self.inner.lock().unwrap();
        world.count_entities_with_type(&type_str)
    }
}

#[pymodule]
fn mge(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWorld>()?;
    Ok(())
}
