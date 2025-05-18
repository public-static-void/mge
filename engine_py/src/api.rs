use engine_core::ecs::world::World;
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::job::JobSystem;
use engine_core::systems::standard::{DamageAll, MoveAll, MoveDelta, ProcessDeaths, ProcessDecay};
use engine_core::worldgen::WorldgenRegistry;
use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use serde_json::Value;
use serde_pyobject::{from_pyobject, to_pyobject};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::event_bus;
use crate::system_bridge::SystemBridge;
use crate::worldgen_bridge::WorldgenBridge;

#[pyclass(unsendable)]
pub struct PyWorld {
    pub inner: Rc<RefCell<World>>,
    pub systems: Rc<SystemBridge>,
    pub worldgen: Rc<WorldgenBridge>,
}

#[pymethods]
impl PyWorld {
    #[new]
    #[pyo3(signature = (schema_dir=None))]
    fn new(schema_dir: Option<String>) -> PyResult<Self> {
        use engine_core::ecs::registry::ComponentRegistry;
        use engine_core::ecs::schema::load_schemas_from_dir;
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

        let mut world = World::new(Arc::new(Mutex::new(registry)));

        // Always initialize a map for the world (so add_cell and movement will work)
        let grid = SquareGridMap::new();
        let map = Map::new(Box::new(grid));
        world.map = Some(map);

        world.register_system(MoveAll {
            delta: MoveDelta::Square {
                dx: 1,
                dy: 0,
                dz: 0,
            },
        });
        world.register_system(DamageAll { amount: 1.0 });
        world.register_system(ProcessDeaths);
        world.register_system(ProcessDecay);
        world.register_system(JobSystem::default());
        Ok(PyWorld {
            inner: Rc::new(RefCell::new(world)),
            systems: Rc::new(SystemBridge {
                systems: RefCell::new(HashMap::new()),
            }),
            worldgen: Rc::new(WorldgenBridge {
                worldgen_registry: RefCell::new(WorldgenRegistry::new()),
            }),
        })
    }

    // --- ECS methods ---

    fn spawn_entity(&self) -> u32 {
        let mut world = self.inner.borrow_mut();
        world.spawn_entity()
    }

    fn despawn_entity(&self, entity_id: u32) {
        let mut world = self.inner.borrow_mut();
        world.despawn_entity(entity_id);
        world.entities.retain(|&e| e != entity_id);
    }

    fn set_component(&self, entity_id: u32, name: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
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
        let world = self.inner.borrow_mut();
        if let Some(val) = world.get_component(entity_id, &name) {
            let py_obj = to_pyobject(py, val)?;
            Ok(Some(py_obj.into()))
        } else {
            Ok(None)
        }
    }

    fn remove_component(&self, entity_id: u32, name: String) {
        let mut world = self.inner.borrow_mut();
        if let Some(comps) = world.components.get_mut(&name) {
            comps.remove(&entity_id);
        }
    }

    fn get_entities_with_component(&self, name: String) -> PyResult<Vec<u32>> {
        let world = self.inner.borrow_mut();
        Ok(world.get_entities_with_component(&name))
    }

    fn get_entities_with_components(&self, names: Vec<String>) -> Vec<u32> {
        let world = self.inner.borrow_mut();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        world.get_entities_with_components(&name_refs)
    }

    fn get_entities(&self) -> PyResult<Vec<u32>> {
        let world = self.inner.borrow_mut();
        Ok(world.get_entities())
    }

    fn is_entity_alive(&self, entity_id: u32) -> bool {
        let world = self.inner.borrow_mut();
        world.is_entity_alive(entity_id)
    }

    fn set_mode(&self, mode: String) {
        let mut world = self.inner.borrow_mut();
        world.current_mode = mode;
    }

    fn get_mode(&self) -> String {
        let world = self.inner.borrow_mut();
        world.current_mode.clone()
    }

    fn get_available_modes(&self) -> Vec<String> {
        let world = self.inner.borrow_mut();
        world
            .registry
            .lock()
            .unwrap()
            .all_modes()
            .into_iter()
            .collect()
    }

    fn list_components(&self) -> Vec<String> {
        let world = self.inner.borrow_mut();
        world.registry.lock().unwrap().all_component_names()
    }

    fn get_component_schema(&self, name: String) -> PyResult<PyObject> {
        let world = self.inner.borrow_mut();
        if let Some(schema) = world.registry.lock().unwrap().get_schema_by_name(&name) {
            let json = serde_json::to_value(&schema.schema)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            Python::with_gil(|py| Ok(to_pyobject(py, &json)?.into()))
        } else {
            Err(PyValueError::new_err("Component schema not found"))
        }
    }

    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32) {
        let mut world = self.inner.borrow_mut();
        world.move_entity(entity_id, dx, dy);
    }

    fn move_all(&self, dx: i32, dy: i32) {
        let mut world = self.inner.borrow_mut();
        world.register_system(MoveAll {
            delta: MoveDelta::Square { dx, dy, dz: 0 },
        });
        world.run_system("MoveAll", None).unwrap();
    }

    /// Add a cell to the map (only works for SquareGridMap).
    fn add_cell(&self, x: i32, y: i32, z: i32) {
        let mut world = self.inner.borrow_mut();
        if let Some(map) = &mut world.map {
            // Try downcasting to SquareGridMap
            if let Some(square) = map
                .topology
                .as_any_mut()
                .downcast_mut::<engine_core::map::SquareGridMap>()
            {
                square.add_cell(x, y, z);
            }
        }
    }

    fn damage_entity(&self, entity_id: u32, amount: f32) {
        let mut world = self.inner.borrow_mut();
        world.damage_entity(entity_id, amount);
    }

    fn damage_all(&self, amount: f32) {
        let mut world = self.inner.borrow_mut();
        world.register_system(DamageAll { amount });
        world.run_system("DamageAll", None).unwrap();
    }

    fn tick(&self) {
        let mut world = self.inner.borrow_mut();
        world.run_system("MoveAll", None).unwrap();
        world.run_system("DamageAll", None).unwrap();
        world.run_system("ProcessDeaths", None).unwrap();
        world.run_system("ProcessDecay", None).unwrap();
        world.turn += 1;
    }

    fn get_turn(&self) -> u32 {
        let world = self.inner.borrow_mut();
        world.turn
    }

    fn process_deaths(&self) {
        let mut world = self.inner.borrow_mut();
        world.register_system(ProcessDeaths);
        world.run_system("ProcessDeaths", None).unwrap();
    }

    fn process_decay(&self) {
        let mut world = self.inner.borrow_mut();
        world.register_system(ProcessDecay);
        world.run_system("ProcessDecay", None).unwrap();
    }

    fn count_entities_with_type(&self, type_str: String) -> usize {
        let world = self.inner.borrow_mut();
        world.count_entities_with_type(&type_str)
    }

    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        world
            .modify_stockpile_resource(entity_id, &kind, delta)
            .map_err(PyValueError::new_err)
    }

    fn save_to_file(&self, path: String) -> PyResult<()> {
        let world = self.inner.borrow_mut();
        world
            .save_to_file(std::path::Path::new(&path))
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn load_from_file(&mut self, path: String) -> PyResult<()> {
        let registry = {
            let world = self.inner.borrow_mut();
            world.registry.clone()
        };
        let loaded = World::load_from_file(std::path::Path::new(&path), registry)
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        let mut world = self.inner.borrow_mut();
        *world = loaded;
        Ok(())
    }

    // --- System registration/bridge ---

    fn register_system(&self, py: Python, name: String, callback: Py<PyAny>) -> PyResult<()> {
        self.systems.register_system(py, name, callback)
    }

    fn run_system(&self, py: Python, name: String) -> PyResult<()> {
        self.systems.run_system(py, name)
    }

    fn run_native_system(&self, name: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        world
            .run_system(&name, None)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    // --- Event bus methods ---

    fn send_event(&self, event_type: String, payload: String) -> PyResult<()> {
        event_bus::send_event(event_type, payload)
    }

    fn poll_event(&self, py: Python, event_type: String) -> PyResult<Vec<PyObject>> {
        event_bus::poll_event(py, event_type)
    }

    fn poll_ecs_event(&self, py: Python, event_type: String) -> PyResult<Vec<PyObject>> {
        let mut world = self.inner.borrow_mut();
        let events = world.take_events(&event_type);
        Ok(events
            .into_iter()
            .map(|e| serde_pyobject::to_pyobject(py, &e).unwrap().into())
            .collect())
    }

    fn update_event_buses(&self) {
        event_bus::update_event_buses()
    }

    // --- User input ---

    fn get_user_input(&self, py: Python, prompt: String) -> PyResult<String> {
        let builtins = py.import("builtins")?;
        let input_func = builtins.getattr("input")?;
        let result = input_func.call1((prompt,))?;
        result.extract::<String>()
    }

    // --- Job system ---

    #[pyo3(signature = (entity_id, job_type, **kwargs))]
    fn assign_job(
        &self,
        entity_id: u32,
        job_type: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let mut job_val = serde_json::json!({
            "job_type": job_type,
            "status": "pending",
            "progress": 0.0
        });
        if let Some(kwargs) = kwargs {
            let extra: serde_json::Value = pythonize::depythonize(kwargs)?;
            if let Some(obj) = extra.as_object() {
                for (k, v) in obj {
                    job_val[k] = v.clone();
                }
            }
        }
        world
            .set_component(entity_id, "Job", job_val)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn register_job_type(&self, _py: Python, name: String, callback: Py<PyAny>) {
        let mut world = self.inner.borrow_mut();
        world.job_types.register_native(
            &name,
            Box::new(move |old_job, progress| {
                Python::with_gil(|py| {
                    let job_obj = to_pyobject(py, old_job).unwrap();
                    let result = callback.call1(py, (job_obj, progress)).unwrap();
                    serde_pyobject::from_pyobject(result.bind(py).clone()).unwrap()
                })
            }),
        );
    }

    // --- Worldgen bridge ---

    fn register_worldgen(&self, py: Python, name: String, callback: Py<PyAny>) -> PyResult<()> {
        self.worldgen.register_worldgen(py, name, callback)
    }

    fn list_worldgen(&self) -> Vec<String> {
        self.worldgen.list_worldgen()
    }

    fn invoke_worldgen<'py>(
        &self,
        py: Python<'py>,
        name: String,
        params: Bound<'py, PyAny>,
    ) -> PyResult<PyObject> {
        self.worldgen.invoke_worldgen(py, name, params)
    }
}
