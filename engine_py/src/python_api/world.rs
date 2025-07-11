use crate::job_bridge::{PY_JOB_HANDLER_REGISTRY, py_job_handler};
use crate::python_api::body::BodyApi;
use crate::python_api::component::ComponentApi;
use crate::python_api::death_decay::DeathDecayApi;
use crate::python_api::economic::EconomicApi;
use crate::python_api::entity::EntityApi;
use crate::python_api::equipment::EquipmentApi;
use crate::python_api::inventory::InventoryApi;
use crate::python_api::job_query::JobQueryApi;
use crate::python_api::mode::ModeApi;
use crate::python_api::region::RegionApi;
use crate::python_api::save_load::SaveLoadApi;
use crate::python_api::time_of_day::TimeOfDayApi;
use crate::python_api::turn::TurnApi;
use crate::system_bridge::SystemBridge;
use engine_core::ecs::world::World;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::types::loader::load_job_types_from_dir;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The main Python-side wrapper for the ECS world.
/// Exposes all core ECS, component, job, inventory, region, and system APIs.
#[pyclass(unsendable, subclass)]
pub struct PyWorld {
    pub inner: Rc<RefCell<World>>,
    pub systems: Rc<SystemBridge>,
    pub map_postprocessors: RefCell<Vec<Py<PyAny>>>,
    pub map_validators: RefCell<Vec<Py<PyAny>>>,
    pub job_handlers: RefCell<HashMap<String, Py<PyAny>>>,
    pub job_board: JobBoard,
}

#[pymethods]
impl PyWorld {
    /// Create a new world, optionally loading schemas from a custom directory.
    #[new]
    #[pyo3(signature = (schema_dir=None))]
    fn new(schema_dir: Option<String>) -> PyResult<Self> {
        use engine_core::ecs::registry::ComponentRegistry;
        use engine_core::ecs::schema::{load_allowed_modes, load_schemas_from_dir_with_modes};
        use std::path::PathBuf;

        let schema_path = match schema_dir {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from("engine/assets/schemas"),
        };

        let allowed_modes = load_allowed_modes().map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to load allowed modes: {e}"))
        })?;
        let schemas =
            load_schemas_from_dir_with_modes(&schema_path, &allowed_modes).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "Failed to load schemas from {schema_path:?}: {e}"
                ))
            })?;

        let mut registry = ComponentRegistry::new();
        for (_name, schema) in schemas {
            registry.register_external_schema(schema);
        }

        let mut world = World::new(std::sync::Arc::new(std::sync::Mutex::new(registry)));

        // Load and register job types from assets
        let jobs_dir = schema_path.parent().unwrap().join("jobs");
        let job_types = load_job_types_from_dir(jobs_dir);
        for job_type in job_types {
            world.job_types.register_job_type(job_type);
        }

        // Always initialize a map for the world (so add_cell and movement will work)
        let grid = engine_core::map::SquareGridMap::new();
        let map = engine_core::map::Map::new(Box::new(grid));
        world.map = Some(map);

        world.register_system(engine_core::systems::death_decay::ProcessDeaths);
        world.register_system(engine_core::systems::death_decay::ProcessDecay);
        world.register_system(engine_core::systems::job::JobSystem);
        Ok(PyWorld {
            inner: Rc::new(RefCell::new(world)),
            systems: Rc::new(SystemBridge {
                systems: RefCell::new(std::collections::HashMap::new()),
            }),
            map_postprocessors: RefCell::new(Vec::new()),
            map_validators: RefCell::new(Vec::new()),
            job_handlers: RefCell::new(HashMap::new()),
            job_board: JobBoard::default(),
        })
    }

    // ---- ENTITY ----

    /// Spawn a new entity and return its ID.
    fn spawn_entity(&self) -> u32 {
        EntityApi::spawn_entity(self)
    }

    /// Despawn (remove) an entity by ID.
    fn despawn_entity(&self, entity_id: u32) {
        EntityApi::despawn_entity(self, entity_id)
    }

    /// Get a list of all entity IDs.
    fn get_entities(&self) -> PyResult<Vec<u32>> {
        EntityApi::get_entities(self)
    }

    /// Count entities with a given type string.
    fn count_entities_with_type(&self, type_str: String) -> usize {
        EntityApi::count_entities_with_type(self, type_str)
    }

    /// Check if an entity is alive.
    fn is_entity_alive(&self, entity_id: u32) -> bool {
        EntityApi::is_entity_alive(self, entity_id)
    }

    /// Move an entity by delta x and y.
    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32) {
        EntityApi::move_entity(self, entity_id, dx, dy)
    }

    /// Apply damage to an entity.
    fn damage_entity(&self, entity_id: u32, amount: f32) {
        EntityApi::damage_entity(self, entity_id, amount)
    }

    // ---- COMPONENT ----

    fn set_component(&self, entity_id: u32, name: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        ComponentApi::set_component(self, entity_id, name, value)
    }

    fn get_component(
        &self,
        py: Python<'_>,
        entity_id: u32,
        name: String,
    ) -> PyResult<Option<PyObject>> {
        ComponentApi::get_component(self, py, entity_id, name)
    }

    fn remove_component(&self, entity_id: u32, name: String) -> PyResult<()> {
        ComponentApi::remove_component(self, entity_id, name)
    }

    fn get_entities_with_component(&self, name: String) -> PyResult<Vec<u32>> {
        ComponentApi::get_entities_with_component(self, name)
    }

    fn get_entities_with_components(&self, names: Vec<String>) -> Vec<u32> {
        ComponentApi::get_entities_with_components(self, names)
    }

    fn list_components(&self) -> Vec<String> {
        ComponentApi::list_components(self)
    }

    fn get_component_schema(&self, name: String) -> PyResult<PyObject> {
        ComponentApi::get_component_schema(self, name)
    }

    // ---- INVENTORY ----

    fn get_inventory(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>> {
        InventoryApi::get_inventory(self, py, entity_id)
    }

    fn set_inventory(&self, entity_id: u32, value: Bound<'_, PyAny>) -> PyResult<()> {
        InventoryApi::set_inventory(self, entity_id, value)
    }

    fn add_item_to_inventory(&self, entity_id: u32, item_id: String) -> PyResult<()> {
        InventoryApi::add_item_to_inventory(self, entity_id, item_id)
    }

    fn remove_item_from_inventory(
        &self,
        py: Python<'_>,
        entity_id: u32,
        index: usize,
    ) -> PyResult<()> {
        InventoryApi::remove_item_from_inventory(self, py, entity_id, index)
    }

    // ---- EQUIPMENT ----

    fn get_equipment(&self, py: Python<'_>, entity_id: u32) -> PyResult<PyObject> {
        EquipmentApi::get_equipment(self, py, entity_id)
    }

    fn equip_item(&self, entity_id: u32, item_id: String, slot: String) -> PyResult<()> {
        EquipmentApi::equip_item(self, entity_id, item_id, slot)
    }

    fn unequip_item(&self, entity_id: u32, slot: String) -> PyResult<()> {
        EquipmentApi::unequip_item(self, entity_id, slot)
    }

    // ---- BODY ----

    fn get_body(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>> {
        BodyApi::get_body(self, py, entity_id)
    }

    fn set_body(&self, entity_id: u32, value: Bound<'_, PyAny>) -> PyResult<()> {
        BodyApi::set_body(self, entity_id, value)
    }

    fn add_body_part(&self, entity_id: u32, part: Bound<'_, PyAny>) -> PyResult<()> {
        BodyApi::add_body_part(self, entity_id, part)
    }

    fn remove_body_part(&self, entity_id: u32, part_name: String) -> PyResult<()> {
        BodyApi::remove_body_part(self, entity_id, part_name)
    }

    fn get_body_part(
        &self,
        py: Python<'_>,
        entity_id: u32,
        part_name: String,
    ) -> PyResult<Option<PyObject>> {
        BodyApi::get_body_part(self, py, entity_id, part_name)
    }

    // ---- REGION ----

    fn get_entities_in_region(&self, region_id: String) -> Vec<u32> {
        RegionApi::get_entities_in_region(self, region_id)
    }

    fn get_entities_in_region_kind(&self, kind: String) -> Vec<u32> {
        RegionApi::get_entities_in_region_kind(self, kind)
    }

    fn get_cells_in_region(&self, py: Python, region_id: String) -> PyResult<PyObject> {
        RegionApi::get_cells_in_region(self, py, region_id)
    }

    fn get_cells_in_region_kind(&self, py: Python, kind: String) -> PyResult<PyObject> {
        RegionApi::get_cells_in_region_kind(self, py, kind)
    }

    // ---- MISC ----

    fn tick(&self) {
        TurnApi::tick(self)
    }

    fn get_turn(&self) -> u32 {
        TurnApi::get_turn(self)
    }

    fn set_mode(&self, mode: String) {
        ModeApi::set_mode(self, mode)
    }

    fn get_mode(&self) -> String {
        ModeApi::get_mode(self)
    }

    fn get_available_modes(&self) -> Vec<String> {
        ModeApi::get_available_modes(self)
    }

    fn process_deaths(&self) {
        DeathDecayApi::process_deaths(self)
    }

    fn process_decay(&self) {
        DeathDecayApi::process_decay(self)
    }

    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()> {
        EconomicApi::modify_stockpile_resource(self, entity_id, kind, delta)
    }

    fn save_to_file(&self, path: String) -> PyResult<()> {
        SaveLoadApi::save_to_file(self, path)
    }

    fn load_from_file(&mut self, path: String) -> PyResult<()> {
        SaveLoadApi::load_from_file(self, path)
    }

    /// Add a cell to the map (utility for tests/scripts)
    fn add_cell(&self, x: i32, y: i32, z: i32) {
        let mut world = self.inner.borrow_mut();
        if let Some(map) = &mut world.map {
            if let Some(square) = map
                .topology
                .as_any_mut()
                .downcast_mut::<engine_core::map::SquareGridMap>()
            {
                square.add_cell(x, y, z);
            }
        }
    }

    // ---- SYSTEM REGISTRATION/BRIDGE ----

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
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    // ---- EVENT BUS ----

    fn send_event(&self, event_type: String, payload: String) -> PyResult<()> {
        crate::event_bus::send_event(event_type, payload)
    }

    fn poll_event(&self, py: Python, event_type: String) -> PyResult<Vec<PyObject>> {
        crate::event_bus::poll_event(py, event_type)
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
        crate::event_bus::update_event_buses()
    }

    // ---- USER INPUT ----

    fn get_user_input(&self, py: Python, prompt: String) -> PyResult<String> {
        let builtins = py.import("builtins")?;
        let input_func = builtins.getattr("input")?;
        let result = input_func.call1((prompt,))?;
        result.extract::<String>()
    }

    // ---- JOB SYSTEM ----

    #[pyo3(signature = (entity_id, job_type, **kwargs))]
    fn assign_job(
        &self,
        entity_id: u32,
        job_type: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let mut job_val = serde_json::json!({
            "id": entity_id,
            "job_type": job_type,
            "state": "pending",
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
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Register a new job type with a Python callback.
    fn register_job_type(&self, py: Python, name: String, callback: Py<PyAny>) {
        PY_JOB_HANDLER_REGISTRY
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name.clone(), callback.clone_ref(py));

        let registry = self.inner.borrow().job_handler_registry.clone();
        registry.lock().unwrap().register_handler(
            &name,
            move |world, agent_id, job_id, job_data| {
                py_job_handler(world, agent_id, job_id, job_data)
            },
        );

        let mut world = self.inner.borrow_mut();
        world
            .job_types
            .register_native(&name, |_world, _agent_id, _job_id, job_data| {
                job_data.clone()
            });
    }

    /// List jobs in the world.
    ///
    /// By default, only active jobs are returned (not in "complete", "failed", or "cancelled" state).
    /// Pass `include_terminal=True` to also include terminal jobs for introspection/analytics.
    #[pyo3(signature = (include_terminal = false))]
    fn list_jobs(
        &self,
        py: pyo3::Python,
        include_terminal: bool,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        JobQueryApi::list_jobs(self, py, Some(include_terminal))
    }

    /// Get a job by ID.
    fn get_job(&self, py: pyo3::Python, job_id: u32) -> pyo3::PyResult<pyo3::PyObject> {
        JobQueryApi::get_job(self, py, job_id)
    }

    /// Find jobs with optional filters.
    #[pyo3(signature = (state=None, job_type=None, assigned_to=None, category=None))]
    fn find_jobs(
        &self,
        py: Python<'_>,
        state: Option<String>,
        job_type: Option<String>,
        assigned_to: Option<u32>,
        category: Option<String>,
    ) -> PyResult<PyObject> {
        JobQueryApi::find_jobs(self, py, state, job_type, assigned_to, category)
    }

    fn get_stockpile_resources(&self, entity_id: u32) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        if let Some(stockpile) = world.get_component(entity_id, "Stockpile") {
            if let Some(resources) = stockpile.get("resources") {
                Python::with_gil(|py| Ok(Some(serde_pyobject::to_pyobject(py, resources)?.into())))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn get_production_job(&self, entity_id: u32) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        if let Some(job) = world.get_component(entity_id, "ProductionJob") {
            Python::with_gil(|py| Ok(Some(serde_pyobject::to_pyobject(py, job)?.into())))
        } else {
            Ok(None)
        }
    }

    /// Get the progress value for a production job by entity ID.
    fn get_production_job_progress(&self, entity_id: u32) -> PyResult<i64> {
        let world = self.inner.borrow();
        if let Some(job) = world.get_component(entity_id, "ProductionJob") {
            Ok(job.get("progress").and_then(|v| v.as_i64()).unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    /// Set the progress value for a production job by entity ID.
    fn set_production_job_progress(&self, entity_id: u32, value: i64) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        if let Some(mut job) = world.get_component(entity_id, "ProductionJob").cloned() {
            job["progress"] = serde_json::json!(value);
            world
                .set_component(entity_id, "ProductionJob", job)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        }
        Ok(())
    }

    /// Get the state string for a production job by entity ID.
    fn get_production_job_state(&self, entity_id: u32) -> PyResult<String> {
        let world = self.inner.borrow();
        if let Some(job) = world.get_component(entity_id, "ProductionJob") {
            Ok(job
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("pending")
                .to_string())
        } else {
            Ok("pending".to_string())
        }
    }

    /// Set the state string for a production job by entity ID.
    fn set_production_job_state(&self, entity_id: u32, value: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        if let Some(mut job) = world.get_component(entity_id, "ProductionJob").cloned() {
            job["state"] = serde_json::json!(value);
            world
                .set_component(entity_id, "ProductionJob", job)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        }
        Ok(())
    }

    /// Returns a list of all registered job type names.
    fn get_job_types(&self) -> PyResult<Vec<String>> {
        let world = self.inner.borrow();
        Ok(world
            .job_types
            .job_type_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect())
    }

    /// Get the metadata for a job type by name.
    /// Returns the job type data as a Python dict, or None if not found.
    fn get_job_type_metadata(&self, py: pyo3::Python, name: String) -> PyResult<Option<PyObject>> {
        let world = self.inner.borrow();
        if let Some(data) = world.job_types.get_data(&name) {
            Ok(Some(serde_pyobject::to_pyobject(py, data)?.into()))
        } else {
            Ok(None)
        }
    }

    fn set_job_field(&self, job_id: u32, field: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        JobQueryApi::set_job_field(self, job_id, &field, &value)
    }

    #[pyo3(signature = (job_id, **kwargs))]
    fn update_job(
        &self,
        job_id: u32,
        kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<()> {
        JobQueryApi::update_job(self, job_id, kwargs)
    }

    fn cancel_job(&self, job_id: u32) -> PyResult<()> {
        JobQueryApi::cancel_job(self, job_id)
    }

    /// Advance the state machine of a single job by its job_id.
    fn advance_job_state(&self, job_id: u32) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let job = match world.get_component(job_id, "Job") {
            Some(job) => job.clone(),
            None => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "No job with id {job_id}"
                )));
            }
        };
        let new_job =
            engine_core::systems::job::system::process::process_job(&mut world, None, job_id, job);
        world.set_component(job_id, "Job", new_job).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to set job: {e}"))
        })?;
        Ok(())
    }

    /// Get the children array (list of job objects) for a job by ID.
    fn get_job_children(&self, py: pyo3::Python, job_id: u32) -> pyo3::PyResult<pyo3::PyObject> {
        let world = self.inner.borrow();
        let job = world.get_component(job_id, "Job").ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!("No job with id {job_id}"))
        })?;
        let children = job
            .get("children")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        Ok(serde_pyobject::to_pyobject(py, &children)?.into())
    }

    /// Set the children array (list of job objects) for a job by ID.
    fn set_job_children(&self, job_id: u32, children: Bound<'_, PyAny>) -> PyResult<()> {
        let children_json: serde_json::Value = serde_pyobject::from_pyobject(children)?;
        let mut world = self.inner.borrow_mut();
        let mut job = world.get_component(job_id, "Job").cloned().ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!("No job with id {job_id}"))
        })?;
        job["children"] = children_json;
        world.set_component(job_id, "Job", job).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to set job: {e}"))
        })?;
        Ok(())
    }

    /// Get the dependencies field for a job by ID.
    fn get_job_dependencies(
        &self,
        py: pyo3::Python,
        job_id: u32,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let world = self.inner.borrow();
        let job = world.get_component(job_id, "Job").ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!("No job with id {job_id}"))
        })?;
        let deps = job
            .get("dependencies")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        Ok(serde_pyobject::to_pyobject(py, &deps)?.into())
    }

    /// Set the dependencies field for a job by ID.
    fn set_job_dependencies(&self, job_id: u32, dependencies: Bound<'_, PyAny>) -> PyResult<()> {
        let deps_json: serde_json::Value = serde_pyobject::from_pyobject(dependencies)?;
        let mut world = self.inner.borrow_mut();
        let mut job = world.get_component(job_id, "Job").cloned().ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!("No job with id {job_id}"))
        })?;
        job["dependencies"] = deps_json;
        world.set_component(job_id, "Job", job).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to set job: {e}"))
        })?;
        Ok(())
    }

    /// Get the current job board as a list of job dicts (eid, priority, state, ...).
    fn get_job_board(&self, py: Python<'_>) -> PyResult<PyObject> {
        let mut world = self.inner.borrow_mut();
        // Using a raw pointer to allow passing `&World` to methods on a field of `World`
        // while holding a mutable borrow, which is safe here because the pointer is not leaked
        // and all access occurs within this single-threaded context.
        let world_ptr: *mut World = &mut *world;
        unsafe {
            world.job_board.update(&*world_ptr);
            let entries = world.job_board.jobs_with_metadata(&*world_ptr);
            let py_entries = PyList::empty(py);
            for entry in entries {
                let dict = PyDict::new(py);
                dict.set_item("eid", entry.eid)?;
                dict.set_item("priority", entry.priority)?;
                dict.set_item("state", entry.state)?;
                py_entries.append(dict)?;
            }
            Ok(py_entries.into())
        }
    }

    /// Get the current job board scheduling policy as a string.
    fn get_job_board_policy(&self) -> String {
        let world = self.inner.borrow();
        world.job_board.get_policy_name().to_string()
    }

    /// Set the job board scheduling policy ("priority", "fifo", "lifo").
    fn set_job_board_policy(&self, policy: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        world
            .job_board
            .set_policy(&policy)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(())
    }

    /// Get the priority value for a job by ID.
    fn get_job_priority(&self, job_id: u32) -> Option<i64> {
        let world = self.inner.borrow();
        world.job_board.get_priority(&world, job_id)
    }

    /// Set the priority for a job by ID.
    fn set_job_priority(&self, job_id: u32, value: i64) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        // Using a raw pointer to avoid borrow checker conflicts when passing a mutable reference
        // to `world` into a method of a field of `world`. This is safe here because the pointer
        // is not leaked and all access is confined to this scope.
        let world_ptr: *mut World = &mut *world;
        world
            .job_board
            .set_priority(unsafe { &mut *world_ptr }, job_id, value)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(())
    }

    // --- Job Event Log Querying ---
    fn get_job_event_log(&self, py: Python) -> PyResult<PyObject> {
        crate::python_api::job_events::get_job_event_log(py)
    }
    fn get_job_events_by_type(&self, py: Python, event_type: String) -> PyResult<PyObject> {
        crate::python_api::job_events::get_job_events_by_type(py, event_type)
    }
    fn get_job_events_since(&self, py: Python, timestamp: u128) -> PyResult<PyObject> {
        crate::python_api::job_events::get_job_events_since(py, timestamp)
    }
    fn get_job_events_where(&self, py: Python, predicate: Bound<'_, PyAny>) -> PyResult<PyObject> {
        crate::python_api::job_events::get_job_events_where(py, predicate)
    }

    // --- Job Event Bus Polling and Subscription ---
    fn poll_job_event_bus(&self, py: Python, event_type: String) -> PyResult<PyObject> {
        let mut world = self.inner.borrow_mut();
        crate::python_api::job_events::poll_job_event_bus(py, event_type, &mut world)
    }
    fn subscribe_job_event_bus(
        &self,
        py: Python,
        event_type: String,
        callback: Py<PyAny>,
    ) -> PyResult<usize> {
        crate::python_api::job_events::subscribe_job_event_bus(py, event_type, callback)
    }
    fn unsubscribe_job_event_bus(&self, event_type: String, sub_id: usize) -> PyResult<()> {
        crate::python_api::job_events::unsubscribe_job_event_bus(event_type, sub_id)
    }

    /// Save the job event log to a file.
    fn save_job_event_log(&self, path: String) -> PyResult<()> {
        crate::python_api::job_events::save_job_event_log_py(path)
    }

    /// Load the job event log from a file.
    fn load_job_event_log(&self, path: String) -> PyResult<()> {
        crate::python_api::job_events::load_job_event_log_py(path)
    }

    /// Replay the job event log into the world.
    fn replay_job_event_log(&self) -> PyResult<()> {
        crate::python_api::job_events::replay_job_event_log_py(self)
    }

    /// Clear the job event log.
    fn clear_job_event_log(&self) -> PyResult<()> {
        crate::python_api::job_events::clear_job_event_log_py()
    }

    // ---- MAP/CAMERA/TOPOLOGY ----

    fn get_map_topology_type(&self) -> String {
        let world = self.inner.borrow();
        world
            .map
            .as_ref()
            .map(|m| m.topology_type().to_string())
            .unwrap_or_else(|| "none".to_string())
    }

    fn get_all_cells(&self, py: Python) -> PyObject {
        let world = self.inner.borrow();
        let cells = world
            .map
            .as_ref()
            .map(|m| m.all_cells())
            .unwrap_or_default();
        serde_pyobject::to_pyobject(py, &cells).unwrap().into()
    }

    fn get_neighbors(&self, py: Python, cell: &Bound<'_, PyAny>) -> PyObject {
        let world = self.inner.borrow();
        let cell_key: engine_core::map::CellKey = pythonize::depythonize(cell).unwrap();
        let neighbors = world
            .map
            .as_ref()
            .map(|m| m.neighbors(&cell_key))
            .unwrap_or_default();
        serde_pyobject::to_pyobject(py, &neighbors).unwrap().into()
    }

    fn add_neighbor(&self, from: (i32, i32, i32), to: (i32, i32, i32)) {
        let mut world = self.inner.borrow_mut();
        if let Some(map) = &mut world.map {
            if let Some(square) = map
                .topology
                .as_any_mut()
                .downcast_mut::<engine_core::map::SquareGridMap>()
            {
                square.add_neighbor(from, to);
            }
        }
    }

    fn entities_in_cell(&self, py: Python, cell: &Bound<'_, PyAny>) -> PyObject {
        let world = self.inner.borrow();
        let cell_key: engine_core::map::CellKey = pythonize::depythonize(cell).unwrap();
        let entities = world.entities_in_cell(&cell_key);
        entities.into_pyobject(py).unwrap().into()
    }

    fn get_cell_metadata(&self, py: Python, cell: &Bound<'_, PyAny>) -> PyObject {
        let world = self.inner.borrow();
        let cell_key: engine_core::map::CellKey = pythonize::depythonize(cell).unwrap();
        if let Some(meta) = world.get_cell_metadata(&cell_key) {
            serde_pyobject::to_pyobject(py, meta).unwrap().into()
        } else {
            py.None()
        }
    }

    fn set_cell_metadata(&self, cell: &Bound<'_, PyAny>, metadata: &Bound<'_, PyAny>) {
        let mut world = self.inner.borrow_mut();
        let cell_key: engine_core::map::CellKey = pythonize::depythonize(cell).unwrap();
        let meta_json: serde_json::Value = pythonize::depythonize(metadata).unwrap();
        world.set_cell_metadata(&cell_key, meta_json);
    }

    fn find_path(&self, py: Python, start: &Bound<'_, PyAny>, goal: &Bound<'_, PyAny>) -> PyObject {
        let world = self.inner.borrow();
        let start_key: engine_core::map::CellKey = pythonize::depythonize(start).unwrap();
        let goal_key: engine_core::map::CellKey = pythonize::depythonize(goal).unwrap();
        if let Some(result) = world.find_path(&start_key, &goal_key) {
            let dict = PyDict::new(py);
            dict.set_item(
                "path",
                serde_pyobject::to_pyobject(py, &result.path).unwrap(),
            )
            .unwrap();
            dict.set_item("total_cost", result.total_cost).unwrap();
            dict.into()
        } else {
            py.None()
        }
    }

    fn register_map_validator(&self, py: Python, callback: Py<PyAny>) {
        self.map_validators
            .borrow_mut()
            .push(callback.clone_ref(py));
    }

    fn clear_map_validators(&self) {
        self.map_validators.borrow_mut().clear();
    }

    fn apply_generated_map<'py>(slf: Bound<'py, Self>, map: Bound<'py, PyAny>) -> PyResult<()> {
        let map_json: serde_json::Value = pythonize::depythonize(&map)?;

        {
            let slf_borrow = slf.borrow();
            let validators = slf_borrow.map_validators.borrow();
            for callback in validators.iter() {
                let ok: bool = callback
                    .call1(slf.py(), (map.clone(),))?
                    .extract(slf.py())?;
                if !ok {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "Map validator failed",
                    ));
                }
            }
        }

        {
            let slf_borrow = slf.borrow();
            let mut world = slf_borrow.inner.borrow_mut();
            world
                .apply_generated_map(&map_json)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
        }

        {
            let slf_borrow = slf.borrow();
            let postprocessors = slf_borrow.map_postprocessors.borrow();
            for callback in postprocessors.iter() {
                callback.call1(slf.py(), (slf.clone(),))?;
            }
        }
        Ok(())
    }

    fn get_map_cell_count(&self) -> usize {
        let world = self.inner.borrow();
        world.map.as_ref().map(|m| m.all_cells().len()).unwrap_or(0)
    }

    /// Register a Python map postprocessor (called after apply_generated_map).
    fn register_map_postprocessor(&self, py: Python, callback: Py<PyAny>) {
        self.map_postprocessors
            .borrow_mut()
            .push(callback.clone_ref(py));
    }

    /// Clear all registered Python map postprocessors.
    fn clear_map_postprocessors(&self) {
        self.map_postprocessors.borrow_mut().clear();
    }

    /// Apply a chunk
    fn apply_chunk<'py>(slf: Bound<'py, Self>, chunk: Bound<'py, PyAny>) -> PyResult<()> {
        let chunk_json: serde_json::Value = pythonize::depythonize(&chunk)?;
        let binding = slf.borrow();
        let mut world = binding.inner.borrow_mut();
        world
            .apply_chunk(&chunk_json)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn get_time_of_day(&self, py: Python) -> PyObject {
        TimeOfDayApi::get_time_of_day(self, py)
    }

    /// Set the camera position (creates camera entity if not present)
    fn set_camera(&self, x: i64, y: i64) {
        let mut world = self.inner.borrow_mut();
        // Find or create the camera entity
        let camera_id = world
            .get_entities_with_component("Camera")
            .first()
            .cloned()
            .unwrap_or_else(|| {
                let id = world.spawn_entity();
                world
                    .set_component(id, "Camera", serde_json::json!({ "x": x, "y": y }))
                    .unwrap();
                id
            });
        // Always update Camera component with x and y
        world
            .set_component(camera_id, "Camera", serde_json::json!({ "x": x, "y": y }))
            .unwrap();
        world
            .set_component(
                camera_id,
                "Position",
                serde_json::json!({ "pos": { "Square": { "x": x, "y": y, "z": 0 } } }),
            )
            .unwrap();
    }

    /// Get the camera position as a dict {x, y}
    fn get_camera(&self, py: Python) -> PyObject {
        let world = self.inner.borrow();
        if let Some(camera_id) = world.get_entities_with_component("Camera").first() {
            if let Some(pos) = world.get_component(*camera_id, "Position") {
                let x = pos["pos"]["Square"]["x"].as_i64().unwrap_or(0);
                let y = pos["pos"]["Square"]["y"].as_i64().unwrap_or(0);
                let dict = PyDict::new(py);
                dict.set_item("x", x).unwrap();
                dict.set_item("y", y).unwrap();
                return dict.into();
            }
        }
        py.None()
    }
}
