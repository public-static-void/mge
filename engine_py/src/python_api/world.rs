use crate::python_api::economic::EconomicApi;
use crate::python_api::mode::ModeApi;
use crate::python_api::turn::TurnApi;
use crate::system_bridge::SystemBridge;
use engine_core::ecs::world::World;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use std::cell::RefCell;
use std::rc::Rc;

// Bring all trait APIs into scope
use crate::python_api::body::BodyApi;
use crate::python_api::component::ComponentApi;
use crate::python_api::entity::EntityApi;
use crate::python_api::equipment::EquipmentApi;
use crate::python_api::inventory::InventoryApi;
use crate::python_api::misc::MiscApi;
use crate::python_api::region::RegionApi;

#[pyclass(unsendable)]
pub struct PyWorld {
    pub inner: Rc<RefCell<World>>,
    pub systems: Rc<SystemBridge>,
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
            pyo3::exceptions::PyValueError::new_err(format!(
                "Failed to load schemas from {:?}: {e}",
                schema_path
            ))
        })?;

        let mut registry = ComponentRegistry::new();
        for (_name, schema) in schemas {
            registry.register_external_schema(schema);
        }

        let mut world = World::new(std::sync::Arc::new(std::sync::Mutex::new(registry)));

        // Always initialize a map for the world (so add_cell and movement will work)
        let grid = engine_core::map::SquareGridMap::new();
        let map = engine_core::map::Map::new(Box::new(grid));
        world.map = Some(map);

        world.register_system(engine_core::systems::death_decay::ProcessDeaths);
        world.register_system(engine_core::systems::death_decay::ProcessDecay);
        world.register_system(engine_core::systems::job::JobSystem::default());
        Ok(PyWorld {
            inner: Rc::new(RefCell::new(world)),
            systems: Rc::new(SystemBridge {
                systems: RefCell::new(std::collections::HashMap::new()),
            }),
        })
    }

    // ---- ENTITY ----
    fn spawn_entity(&self) -> u32 {
        EntityApi::spawn_entity(self)
    }
    fn despawn_entity(&self, entity_id: u32) {
        EntityApi::despawn_entity(self, entity_id)
    }
    fn get_entities(&self) -> PyResult<Vec<u32>> {
        EntityApi::get_entities(self)
    }
    fn is_entity_alive(&self, entity_id: u32) -> bool {
        EntityApi::is_entity_alive(self, entity_id)
    }
    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32) {
        EntityApi::move_entity(self, entity_id, dx, dy)
    }
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
    fn remove_component(&self, entity_id: u32, name: String) {
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
        MiscApi::process_deaths(self)
    }
    fn process_decay(&self) {
        MiscApi::process_decay(self)
    }
    fn count_entities_with_type(&self, type_str: String) -> usize {
        MiscApi::count_entities_with_type(self, type_str)
    }
    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()> {
        EconomicApi::modify_stockpile_resource(self, entity_id, kind, delta)
    }
    fn save_to_file(&self, path: String) -> PyResult<()> {
        MiscApi::save_to_file(self, path)
    }
    fn load_from_file(&mut self, path: String) -> PyResult<()> {
        MiscApi::load_from_file(self, path)
    }

    // ---- ADDITIONAL METHODS ----

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
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn register_job_type(&self, _py: Python, name: String, callback: Py<PyAny>) {
        let mut world = self.inner.borrow_mut();
        world.job_types.register_native(
            &name,
            Box::new(move |old_job, progress| {
                Python::with_gil(|py| {
                    let job_obj = serde_pyobject::to_pyobject(py, old_job).unwrap();
                    let result = callback.call1(py, (job_obj, progress)).unwrap();
                    serde_pyobject::from_pyobject(result.bind(py).clone()).unwrap()
                })
            }),
        );
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

    /// Returns a list of all registered job type names.
    fn get_job_types(&self) -> PyResult<Vec<String>> {
        let world = self.inner.borrow();
        Ok(world.job_types.job_type_names())
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

    fn get_time_of_day(&self, py: Python) -> PyObject {
        let world = self.inner.borrow();
        let tod = world.get_time_of_day();
        let dict = PyDict::new(py);
        dict.set_item("hour", tod.hour).unwrap();
        dict.set_item("minute", tod.minute).unwrap();
        dict.into_pyobject(py).unwrap().unbind().into()
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
