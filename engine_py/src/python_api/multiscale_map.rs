use crate::PyObject;
use crate::python_api::world::PyWorld;
use engine_core::map::Map;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pythonize::depythonize;

/// Register a named map from a GeneratedMap JSON dict. Error on duplicate name.
///
/// `map` is a Python dict with the `map.json` shape (`topology` + `cells`).
pub fn register_map(pyworld: &PyWorld, name: String, map: &Bound<'_, PyAny>) -> PyResult<()> {
    let map_json: serde_json::Value = depythonize(map)?;
    let map = Map::from_json(&map_json).map_err(pyo3::exceptions::PyValueError::new_err)?;
    let mut world = pyworld.inner.borrow_mut();
    world
        .register_map(&name, map)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

/// Set the active map to a registered map. Error on unknown name.
pub fn set_active_map(pyworld: &PyWorld, name: String) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    world
        .set_active_map(&name)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

/// List registered map names.
pub fn get_map_names(pyworld: &PyWorld) -> Vec<String> {
    let world = pyworld.inner.borrow();
    world.get_map_names()
}

/// Current active map name.
pub fn get_active_map_name(pyworld: &PyWorld) -> String {
    let world = pyworld.inner.borrow();
    world.get_active_map_name()
}

/// Link a source map cell to a target map cell. Error on unknown map name.
///
/// `source_cell`/`target_cell` are dicts in the Position shape
/// (`{"Square": {"x", "y", "z"}}` / `{"Hex": {"q", "r", "z"}}` /
/// `{"Province": {"id"}}`).
pub fn link_maps(
    pyworld: &PyWorld,
    source_map: String,
    source_cell: &Bound<'_, PyAny>,
    target_map: String,
    target_cell: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let source_key: engine_core::map::CellKey = depythonize(source_cell)?;
    let target_key: engine_core::map::CellKey = depythonize(target_cell)?;
    let mut world = pyworld.inner.borrow_mut();
    world
        .link_maps(&source_map, source_key, &target_map, target_key)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

/// Transition to the named map, camera at entry cell. Error on unknown map name.
pub fn enter_map(pyworld: &PyWorld, name: String, entry_cell: &Bound<'_, PyAny>) -> PyResult<()> {
    let entry_key: engine_core::map::CellKey = depythonize(entry_cell)?;
    let mut world = pyworld.inner.borrow_mut();
    world
        .enter_map(&name, entry_key)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

/// Return to the previously active map. Error if no prior map.
pub fn exit_map(pyworld: &PyWorld) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    world
        .exit_map()
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

/// Map a source cell to the linked target cell. None if unlinked.
pub fn map_cell(
    pyworld: &PyWorld,
    py: Python,
    source_map: String,
    source_cell: &Bound<'_, PyAny>,
) -> PyObject {
    let source_key: engine_core::map::CellKey = match depythonize(source_cell) {
        Ok(v) => v,
        Err(_) => return py.None(),
    };
    let world = pyworld.inner.borrow();
    match world.map_cell(&source_map, &source_key) {
        Some(cell) => serde_pyobject::to_pyobject(py, &cell).unwrap().into(),
        None => py.None(),
    }
}

/// Reverse-map a target cell to the linked source cell. None if unlinked.
pub fn unmap_cell(
    pyworld: &PyWorld,
    py: Python,
    target_map: String,
    target_cell: &Bound<'_, PyAny>,
) -> PyObject {
    let target_key: engine_core::map::CellKey = match depythonize(target_cell) {
        Ok(v) => v,
        Err(_) => return py.None(),
    };
    let world = pyworld.inner.borrow();
    match world.unmap_cell(&target_map, &target_key) {
        Some(cell) => serde_pyobject::to_pyobject(py, &cell).unwrap().into(),
        None => py.None(),
    }
}
