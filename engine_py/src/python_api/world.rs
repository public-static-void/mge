use crate::python_api::body::BodyApi;
use crate::python_api::component::ComponentApi;
use crate::python_api::death_decay::DeathDecayApi;
use crate::python_api::economic::EconomicApi;
use crate::python_api::entity::EntityApi;
use crate::python_api::equipment::EquipmentApi;
use crate::python_api::inventory::InventoryApi;
use crate::python_api::job_query::JobQueryApi;
use crate::python_api::mode::ModeApi;
use crate::python_api::movement::MovementApi;
use crate::python_api::region::RegionApi;
use crate::python_api::save_load::SaveLoadApi;
use crate::python_api::time_of_day::TimeOfDayApi;
use crate::python_api::turn::TurnApi;
use crate::system_bridge::SystemBridge;
use engine_core::ecs::world::World;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::types::loader::load_job_types_from_dir;
use pyo3::Python;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyAnyMethods, PyDict, PyList};
use pythonize::depythonize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The main Python-side wrapper for the ECS world.
/// Exposes all core ECS, component, job, inventory, region, and system APIs.
#[pyclass(unsendable, subclass)]
pub struct PyWorld {
    pub inner: Rc<RefCell<World>>,
    pub systems: Rc<SystemBridge>,
    pub map_postprocessors: Rc<RefCell<Vec<Py<PyAny>>>>,
    pub map_validators: Rc<RefCell<Vec<Py<PyAny>>>>,
    pub job_handlers: Rc<RefCell<HashMap<String, Py<PyAny>>>>,
    pub job_board: Rc<RefCell<JobBoard>>,
}

// Manual Clone implementation using Rc clones
impl Clone for PyWorld {
    fn clone(&self) -> Self {
        PyWorld {
            inner: self.inner.clone(),
            systems: self.systems.clone(),
            map_postprocessors: self.map_postprocessors.clone(),
            map_validators: self.map_validators.clone(),
            job_handlers: self.job_handlers.clone(),
            job_board: self.job_board.clone(),
        }
    }
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
            map_postprocessors: Rc::new(RefCell::new(Vec::new())),
            map_validators: Rc::new(RefCell::new(Vec::new())),
            job_handlers: Rc::new(RefCell::new(HashMap::new())),
            job_board: Rc::new(RefCell::new(JobBoard::default())),
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

    // Set component
    fn set_component(&self, entity_id: u32, name: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        ComponentApi::set_component(self, entity_id, name, value)
    }

    // Get component
    fn get_component(
        &self,
        py: Python<'_>,
        entity_id: u32,
        name: String,
    ) -> PyResult<Option<PyObject>> {
        ComponentApi::get_component(self, py, entity_id, name)
    }

    // Remove component
    fn remove_component(&self, entity_id: u32, name: String) -> PyResult<()> {
        ComponentApi::remove_component(self, entity_id, name)
    }

    // Get all entities with a given component
    fn get_entities_with_component(&self, name: String) -> PyResult<Vec<u32>> {
        ComponentApi::get_entities_with_component(self, name)
    }

    // Get all entities with a given list of components
    fn get_entities_with_components(&self, names: Vec<String>) -> Vec<u32> {
        ComponentApi::get_entities_with_components(self, names)
    }

    // List all components
    fn list_components(&self) -> Vec<String> {
        ComponentApi::list_components(self)
    }

    // Get component schema
    fn get_component_schema(&self, name: String) -> PyResult<PyObject> {
        ComponentApi::get_component_schema(self, name)
    }

    // ---- INVENTORY ----

    // Get inventory
    fn get_inventory(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>> {
        InventoryApi::get_inventory(self, py, entity_id)
    }

    // Set inventory
    fn set_inventory(&self, entity_id: u32, value: Bound<'_, PyAny>) -> PyResult<()> {
        InventoryApi::set_inventory(self, entity_id, value)
    }

    // Add item to inventory
    fn add_item_to_inventory(&self, entity_id: u32, item_id: String) -> PyResult<()> {
        InventoryApi::add_item_to_inventory(self, entity_id, item_id)
    }

    // Remove item from inventory
    fn remove_item_from_inventory(
        &self,
        py: Python<'_>,
        entity_id: u32,
        index: usize,
    ) -> PyResult<()> {
        InventoryApi::remove_item_from_inventory(self, py, entity_id, index)
    }

    // ---- EQUIPMENT ----

    // Get equipment
    fn get_equipment(&self, py: Python<'_>, entity_id: u32) -> PyResult<PyObject> {
        EquipmentApi::get_equipment(self, py, entity_id)
    }

    // Equip item
    fn equip_item(&self, entity_id: u32, item_id: String, slot: String) -> PyResult<()> {
        EquipmentApi::equip_item(self, entity_id, item_id, slot)
    }

    // Unequip item
    fn unequip_item(&self, entity_id: u32, slot: String) -> PyResult<()> {
        EquipmentApi::unequip_item(self, entity_id, slot)
    }

    // ---- BODY ----

    // Get body
    fn get_body(&self, py: Python<'_>, entity_id: u32) -> PyResult<Option<PyObject>> {
        BodyApi::get_body(self, py, entity_id)
    }

    // Set body
    fn set_body(&self, entity_id: u32, value: Bound<'_, PyAny>) -> PyResult<()> {
        BodyApi::set_body(self, entity_id, value)
    }

    // Add body part
    fn add_body_part(&self, entity_id: u32, part: Bound<'_, PyAny>) -> PyResult<()> {
        BodyApi::add_body_part(self, entity_id, part)
    }

    // Remove body part
    fn remove_body_part(&self, entity_id: u32, part_name: String) -> PyResult<()> {
        BodyApi::remove_body_part(self, entity_id, part_name)
    }

    // Get body part
    fn get_body_part(
        &self,
        py: Python<'_>,
        entity_id: u32,
        part_name: String,
    ) -> PyResult<Option<PyObject>> {
        BodyApi::get_body_part(self, py, entity_id, part_name)
    }

    // ---- REGION ----

    // Get entities in region
    fn get_entities_in_region(&self, region_id: String) -> Vec<u32> {
        RegionApi::get_entities_in_region(self, region_id)
    }

    // Get entities in kind of region
    fn get_entities_in_region_kind(&self, kind: String) -> Vec<u32> {
        RegionApi::get_entities_in_region_kind(self, kind)
    }

    // Get cells in region
    fn get_cells_in_region(&self, py: Python, region_id: String) -> PyResult<PyObject> {
        RegionApi::get_cells_in_region(self, py, region_id)
    }

    // Get cells in kind of region
    fn get_cells_in_region_kind(&self, py: Python, kind: String) -> PyResult<PyObject> {
        RegionApi::get_cells_in_region_kind(self, py, kind)
    }

    // ---- MISC ----

    // Progress the turn
    fn tick(&self) {
        TurnApi::tick(self)
    }

    // Get current turn
    fn get_turn(&self) -> u32 {
        TurnApi::get_turn(self)
    }

    // Set game mode
    fn set_mode(&self, mode: String) {
        ModeApi::set_mode(self, mode)
    }

    // Get game mode
    fn get_mode(&self) -> String {
        ModeApi::get_mode(self)
    }

    // Get available game modes
    fn get_available_modes(&self) -> Vec<String> {
        ModeApi::get_available_modes(self)
    }

    // Process deaths
    fn process_deaths(&self) {
        DeathDecayApi::process_deaths(self)
    }

    // Process decay
    fn process_decay(&self) {
        DeathDecayApi::process_decay(self)
    }

    // Save
    fn save_to_file(&self, path: String) -> PyResult<()> {
        SaveLoadApi::save_to_file(self, path)
    }

    // Load
    fn load_from_file(&mut self, path: String) -> PyResult<()> {
        SaveLoadApi::load_from_file(self, path)
    }

    /// Get the time of day
    fn get_time_of_day(&self, py: Python) -> PyObject {
        TimeOfDayApi::get_time_of_day(self, py)
    }

    /// Add a cell to the map
    fn add_cell(&self, x: i32, y: i32, z: i32) {
        crate::python_api::map_api::add_cell(self, x, y, z)
    }

    // ---- SYSTEM REGISTRATION/BRIDGE ----

    // Register a system
    fn register_system(&self, py: Python, name: String, callback: Py<PyAny>) -> PyResult<()> {
        self.systems.register_system(py, name, callback)
    }

    // Run a system
    fn run_system(&self, py: Python, name: String) -> PyResult<()> {
        self.systems.run_system(py, name)
    }

    // Run a native system
    fn run_native_system(&self, name: String) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        world
            .run_system(&name, None)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    // ---- EVENT BUS ----

    // Send event
    fn send_event(&self, event_type: String, payload: String) -> PyResult<()> {
        crate::event_bus::send_event(event_type, payload)
    }

    // Poll event
    fn poll_event(&self, py: Python, event_type: String) -> PyResult<Vec<PyObject>> {
        crate::event_bus::poll_event(py, event_type)
    }

    // Poll ECS event
    fn poll_ecs_event(&self, py: Python, event_type: String) -> PyResult<Vec<PyObject>> {
        crate::event_bus::poll_ecs_event(self, py, event_type)
    }

    // Update event buses
    fn update_event_buses(&self) {
        crate::event_bus::update_event_buses()
    }

    // ---- USER INPUT ----

    // Get user input
    fn get_user_input(&self, py: Python, prompt: String) -> PyResult<String> {
        let builtins = py.import("builtins")?;
        let input_func = builtins.getattr("input")?;
        let result = input_func.call1((prompt,))?;
        result.extract::<String>()
    }

    // ---- JOB SYSTEM ----

    // Assign a job
    #[pyo3(signature = (entity_id, job_type, **kwargs))]
    fn assign_job(
        &self,
        entity_id: u32,
        job_type: String,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        crate::python_api::job_api::assign_job(self, entity_id, job_type, kwargs)
    }

    /// Register a new job type with a Python callback.
    fn register_job_type(&self, py: Python, name: String, callback: Py<PyAny>) {
        crate::python_api::job_api::register_job_type(self, py, name, callback)
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

    // Get stockpile resources
    fn get_stockpile_resources(&self, entity_id: u32) -> PyResult<Option<PyObject>> {
        crate::python_api::economic::get_stockpile_resources(self, entity_id)
    }

    // Modify stockpile resource
    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()> {
        EconomicApi::modify_stockpile_resource(self, entity_id, kind, delta)
    }

    /// Get a production job by entity ID.
    fn get_production_job(&self, py: Python, entity_id: u32) -> PyResult<Option<PyObject>> {
        crate::python_api::job_production::get_production_job(self, py, entity_id)
    }

    /// Get the progress value for a production job by entity ID.
    fn get_production_job_progress(&self, entity_id: u32) -> PyResult<i64> {
        crate::python_api::job_production::get_production_job_progress(self, entity_id)
    }

    /// Set the progress value for a production job by entity ID.
    fn set_production_job_progress(&self, entity_id: u32, value: i64) -> PyResult<()> {
        crate::python_api::job_production::set_production_job_progress(self, entity_id, value)
    }

    /// Get the state string for a production job by entity ID.
    fn get_production_job_state(&self, entity_id: u32) -> PyResult<String> {
        crate::python_api::job_production::get_production_job_state(self, entity_id)
    }

    /// Set the state string for a production job by entity ID.
    fn set_production_job_state(&self, entity_id: u32, value: String) -> PyResult<()> {
        crate::python_api::job_production::set_production_job_state(self, entity_id, value)
    }

    /// Get the reserved resources for a job by entity ID.
    /// Returns a list of dicts or None.
    fn get_job_resource_reservations(
        &self,
        entity_id: u32,
        py: Python,
    ) -> PyResult<Option<PyObject>> {
        crate::python_api::job_reservation::get_job_resource_reservations(self, entity_id, py)
    }

    /// Reserve job resources
    fn reserve_job_resources(&self, entity_id: u32) -> PyResult<bool> {
        crate::python_api::job_reservation::reserve_job_resources(self, entity_id)
    }

    /// Release job resources
    fn release_job_resource_reservations(&self, entity_id: u32) -> PyResult<()> {
        crate::python_api::job_reservation::release_job_resource_reservations(self, entity_id)
    }

    /// Run the resource reservation system explicitly.
    fn run_resource_reservation_system(&self) -> PyResult<()> {
        crate::python_api::job_reservation::run_resource_reservation_system(self)
    }

    /// Returns a list of all registered job type names.
    fn get_job_types(&self) -> PyResult<Vec<String>> {
        crate::python_api::job_board::get_job_types(self)
    }

    /// Get the metadata for a job type by name.
    /// Returns the job type data as a Python dict, or None if not found.
    fn get_job_type_metadata(&self, py: Python, name: String) -> PyResult<Option<PyObject>> {
        crate::python_api::job_board::get_job_type_metadata(self, py, name)
    }

    // Set a field on a job
    fn set_job_field(&self, job_id: u32, field: String, value: Bound<'_, PyAny>) -> PyResult<()> {
        JobQueryApi::set_job_field(self, job_id, &field, &value)
    }

    // Update a job
    #[pyo3(signature = (job_id, **kwargs))]
    fn update_job(
        &self,
        job_id: u32,
        kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> PyResult<()> {
        JobQueryApi::update_job(self, job_id, kwargs)
    }

    // Cancel a job
    fn cancel_job(&self, job_id: u32) -> PyResult<()> {
        JobQueryApi::cancel_job(self, job_id)
    }

    /// Advance the state machine of a single job by its job_id.
    fn advance_job_state(&self, job_id: u32) -> PyResult<()> {
        crate::python_api::job_api::advance_job_state(self, job_id)
    }

    /// Get the children array (list of job objects) for a job by ID.
    fn get_job_children(&self, py: Python, job_id: u32) -> PyResult<PyObject> {
        crate::python_api::job_children::get_job_children(self, py, job_id)
    }

    /// Set the children array (list of job objects) for a job by ID.
    fn set_job_children(&self, job_id: u32, children: Bound<'_, PyAny>) -> PyResult<()> {
        crate::python_api::job_children::set_job_children(self, job_id, children)
    }

    /// Get the dependencies field for a job by ID.
    fn get_job_dependencies(&self, py: Python, job_id: u32) -> PyResult<PyObject> {
        crate::python_api::job_dependencies::get_job_dependencies(self, py, job_id)
    }

    /// Set the dependencies field for a job by ID.
    fn set_job_dependencies(&self, job_id: u32, dependencies: Bound<'_, PyAny>) -> PyResult<()> {
        crate::python_api::job_dependencies::set_job_dependencies(self, job_id, dependencies)
    }

    /// Get the current job board as a list of job dicts (eid, priority, state, ...).
    fn get_job_board(&self, py: Python) -> PyResult<PyObject> {
        crate::python_api::job_board::get_job_board(self, py)
    }

    /// Get the current job board scheduling policy as a string.
    fn get_job_board_policy(&self) -> String {
        crate::python_api::job_board::get_job_board_policy(self)
    }

    /// Set the job board scheduling policy ("priority", "fifo", "lifo").
    fn set_job_board_policy(&self, policy: String) -> PyResult<()> {
        crate::python_api::job_board::set_job_board_policy(self, policy)
    }

    /// Get the priority value for a job by ID.
    fn get_job_priority(&self, job_id: u32) -> Option<i64> {
        crate::python_api::job_board::get_job_priority(self, job_id)
    }

    /// Set the priority for a job by ID.
    fn set_job_priority(&self, job_id: u32, value: i64) -> PyResult<()> {
        crate::python_api::job_board::set_job_priority(self, job_id, value)
    }

    // --- Job Event Log Querying ---

    // Get the job event log
    fn get_job_event_log(&self, py: Python) -> PyResult<PyObject> {
        crate::python_api::job_events::get_job_event_log(py)
    }

    // Get job events by type
    fn get_job_events_by_type(&self, py: Python, event_type: String) -> PyResult<PyObject> {
        crate::python_api::job_events::get_job_events_by_type(py, event_type)
    }

    // Get job events since
    fn get_job_events_since(&self, py: Python, timestamp: u128) -> PyResult<PyObject> {
        crate::python_api::job_events::get_job_events_since(py, timestamp)
    }

    // Get job events where
    fn get_job_events_where(&self, py: Python, predicate: Bound<'_, PyAny>) -> PyResult<PyObject> {
        crate::python_api::job_events::get_job_events_where(py, predicate)
    }

    // --- Job Event Bus Polling and Subscription ---

    // Poll the job event bus
    fn poll_job_event_bus(&self, py: Python, event_type: String) -> PyResult<PyObject> {
        let mut world = self.inner.borrow_mut();
        crate::python_api::job_events::poll_job_event_bus(py, event_type, &mut world)
    }

    // Subscribe to job event bus
    fn subscribe_job_event_bus(
        &self,
        py: Python,
        event_type: String,
        callback: Py<PyAny>,
    ) -> PyResult<usize> {
        crate::python_api::job_events::subscribe_job_event_bus(py, event_type, callback)
    }

    // Unsubscribe to job event bus
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

    /// Assign a move path to an agent.
    pub fn assign_move_path(
        &self,
        agent_id: u32,
        from_cell: Bound<'_, PyAny>,
        to_cell: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let from_val: serde_json::Value = depythonize(&from_cell)
            .map_err(|e| PyValueError::new_err(format!("Invalid from_cell: {e}")))?;
        let to_val: serde_json::Value = depythonize(&to_cell)
            .map_err(|e| PyValueError::new_err(format!("Invalid to_cell: {e}")))?;

        MovementApi::assign_move_path(self, agent_id, from_val, to_val)
    }

    /// Check if an agent is at a cell.
    pub fn is_agent_at_cell(&self, agent_id: u32, cell: Bound<'_, PyAny>) -> PyResult<bool> {
        let val: serde_json::Value =
            depythonize(&cell).map_err(|e| PyValueError::new_err(format!("Invalid cell: {e}")))?;

        MovementApi::is_agent_at_cell(self, agent_id, val)
    }

    /// Check if an agent's move path is empty.
    pub fn is_move_path_empty(&self, agent_id: u32) -> PyResult<bool> {
        MovementApi::is_move_path_empty(self, agent_id)
    }

    #[pyo3(signature = (agent_id, _args))]
    fn ai_assign_jobs(&self, agent_id: u32, _args: Vec<PyObject>) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();

        let job_board_ptr: *mut _ = &mut world.job_board;
        use engine_core::systems::job::ai::logic::assign_jobs;

        unsafe {
            assign_jobs(&mut world, &mut *job_board_ptr, agent_id as u64, &[]);
        }

        Ok(())
    }

    #[pyo3(signature = (agent_id))]
    fn ai_query_jobs(&self, py: Python, agent_id: u32) -> PyResult<PyObject> {
        let world = self.inner.borrow();
        let mut jobs_py: Vec<Py<PyAny>> = Vec::new();

        if let Some(job_map) = world.components.get("Job") {
            for (&job_id, job_comp) in job_map.iter() {
                if let Some(assigned_to) = job_comp.get("assigned_to").and_then(|v| v.as_u64()) {
                    if assigned_to == agent_id as u64 {
                        let dict = PyDict::new(py);
                        dict.set_item("id", job_id)?;
                        dict.set_item(
                            "state",
                            job_comp.get("state").and_then(|v| v.as_str()).unwrap_or(""),
                        )?;
                        dict.set_item(
                            "job_type",
                            job_comp
                                .get("job_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or(""),
                        )?;
                        dict.set_item("assigned_to", assigned_to)?;
                        jobs_py.push(dict.into());
                    }
                }
            }
        }

        Ok(PyList::new(py, jobs_py)?.into())
    }

    #[pyo3(signature = (job_id, **kwargs))]
    fn ai_modify_job_assignment(
        &self,
        py: Python,
        job_id: u32,
        kwargs: Option<PyObject>,
    ) -> PyResult<bool> {
        let mut world = self.inner.borrow_mut();

        // Get the job component json or error if missing
        let mut job = world
            .get_component(job_id, "Job")
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!("No job with id {job_id}"))
            })?
            .clone();

        if let Some(kwargs_obj) = kwargs {
            // Convert PyObject kwargs to smart borrowed PyDict reference
            let kwargs_dict = kwargs_obj
                .downcast_bound::<PyDict>(py)
                .map_err(|_| pyo3::exceptions::PyValueError::new_err("kwargs must be a dict"))?;

            // Iterate over dict items updating job json
            for (key, value) in kwargs_dict.iter() {
                let k: String = key.extract()?;
                let v: serde_json::Value = depythonize(&value)?; // pass reference as expected
                job[k] = v;
            }
        }

        // Persist updated job component
        world.set_component(job_id, "Job", job).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to set job: {e}"))
        })?;

        Ok(true)
    }

    // ---- MAP/CAMERA/TOPOLOGY ----

    /// Get the topology type of the current map.
    fn get_map_topology_type(&self) -> String {
        crate::python_api::map_api::get_map_topology_type(self)
    }

    /// Get a list of all cells in the current map.
    fn get_all_cells(&self, py: Python) -> PyObject {
        crate::python_api::map_api::get_all_cells(self, py)
    }

    /// Get the neighbors of a given cell.
    fn get_neighbors(&self, py: Python, cell: &Bound<'_, PyAny>) -> PyObject {
        crate::python_api::map_api::get_neighbors(self, py, cell)
    }

    /// Add a directed neighbor edge from one cell to another.
    fn add_neighbor(&self, from: (i32, i32, i32), to: (i32, i32, i32)) {
        crate::python_api::map_api::add_neighbor(self, from, to)
    }

    /// Get a list of entity IDs located in the given cell.
    fn entities_in_cell(&self, py: Python, cell: &Bound<'_, PyAny>) -> PyObject {
        crate::python_api::map_api::entities_in_cell(self, py, cell)
    }

    /// Get metadata associated with a given cell.
    fn get_cell_metadata(&self, py: Python, cell: &Bound<'_, PyAny>) -> PyObject {
        crate::python_api::map_api::get_cell_metadata(self, py, cell)
    }

    /// Set metadata for a given cell.
    fn set_cell_metadata(
        &self,
        cell: &Bound<'_, PyAny>,
        metadata: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        crate::python_api::map_api::set_cell_metadata(self, cell, metadata)
    }

    /// Find a path between two cells using the map's pathfinding system.
    fn find_path(&self, py: Python, start: &Bound<'_, PyAny>, goal: &Bound<'_, PyAny>) -> PyObject {
        crate::python_api::map_api::find_path(self, py, start, goal)
    }

    /// Register a Python callback as a map validator.
    fn register_map_validator(&self, py: Python, callback: Py<PyAny>) {
        crate::python_api::map_api::register_map_validator(self, py, callback)
    }

    /// Clear all registered Python map validators.
    fn clear_map_validators(&self) {
        crate::python_api::map_api::clear_map_validators(self)
    }

    /// Apply a generated map JSON.
    fn apply_generated_map(&self, py: Python<'_>, map: &Bound<'_, PyAny>) -> PyResult<()> {
        let pyworld_obj: Py<PyWorld> = Py::new(py, self.clone())?;
        crate::python_api::map_api::apply_generated_map(pyworld_obj, py, map)
    }

    /// Apply a chunk of map JSON data.
    fn apply_chunk(&self, py: Python<'_>, chunk: &Bound<'_, PyAny>) -> PyResult<()> {
        let pyworld_obj: Py<PyWorld> = Py::new(py, self.clone())?;
        crate::python_api::map_api::apply_chunk(pyworld_obj, py, chunk)
    }

    // Get the number of cells in the current map
    fn get_map_cell_count(&self) -> usize {
        let world = self.inner.borrow();
        world.map.as_ref().map(|m| m.all_cells().len()).unwrap_or(0)
    }

    /// Register a Python callback as a map postprocessor.
    fn register_map_postprocessor(&self, py: Python, callback: Py<PyAny>) {
        crate::python_api::map_api::register_map_postprocessor(self, py, callback)
    }

    /// Clear all registered Python map postprocessors.
    fn clear_map_postprocessors(&self) {
        crate::python_api::map_api::clear_map_postprocessors(self)
    }

    /// Set the camera position (creates camera entity if not present)
    fn set_camera(&self, x: i64, y: i64) {
        crate::python_api::camera_api::set_camera(self, x, y)
    }

    /// Get the camera position as a dict {x, y}
    fn get_camera(&self, py: Python) -> PyObject {
        crate::python_api::camera_api::get_camera(self, py)
    }
}
