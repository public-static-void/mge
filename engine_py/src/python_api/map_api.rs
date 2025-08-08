use crate::python_api::world::PyWorld;
use pyo3::Py;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use pythonize::depythonize;

/// Get the topology type of the current map.
///
/// Returns a string representation of the topology type,
/// or `"none"` if no map is loaded.
pub fn get_map_topology_type(pyworld: &PyWorld) -> String {
    let world = pyworld.inner.borrow();
    world
        .map
        .as_ref()
        .map(|m| m.topology_type().to_string())
        .unwrap_or_else(|| "none".to_string())
}

/// Get a list of all cells in the current map.
///
/// Returns a Python object representing the list of all cells.
/// Returns an empty list if no map is loaded.
pub fn get_all_cells(pyworld: &PyWorld, py: Python) -> PyObject {
    let world = pyworld.inner.borrow();
    let cells = world
        .map
        .as_ref()
        .map(|m| m.all_cells())
        .unwrap_or_default();
    serde_pyobject::to_pyobject(py, &cells).unwrap().into()
}

/// Get the neighbors of a given cell.
///
/// `cell` is a Python object representing a cell key (e.g., coordinates).
/// Returns a Python list of neighbor cells, or an empty list if map or cell not found.
pub fn get_neighbors(pyworld: &PyWorld, py: Python, cell: &Bound<'_, PyAny>) -> PyObject {
    let world = pyworld.inner.borrow();
    let cell_key: engine_core::map::CellKey = match pythonize::depythonize(cell) {
        Ok(val) => val,
        Err(_) => return py.None(),
    };
    let neighbors = world
        .map
        .as_ref()
        .map(|m| m.neighbors(&cell_key))
        .unwrap_or_default();
    serde_pyobject::to_pyobject(py, &neighbors).unwrap().into()
}

/// Add a directed neighbor edge from one cell to another.
///
/// `from` and `to` are tuples of coordinates `(x, y, z)`.
pub fn add_neighbor(pyworld: &PyWorld, from: (i32, i32, i32), to: (i32, i32, i32)) {
    let mut world = pyworld.inner.borrow_mut();
    if let Some(map) = &mut world.map
        && let Some(square) = map
            .topology
            .as_any_mut()
            .downcast_mut::<engine_core::map::SquareGridMap>()
        {
            square.add_neighbor(from, to);
        }
}

/// Get a list of entity IDs located in the given cell.
///
/// `cell` is a Python object representing a cell key.
/// Returns a Python list of entity IDs present in the cell.
pub fn entities_in_cell(pyworld: &PyWorld, py: Python, cell: &Bound<'_, PyAny>) -> PyObject {
    let world = pyworld.inner.borrow();
    let cell_key: engine_core::map::CellKey = match pythonize::depythonize(cell) {
        Ok(val) => val,
        Err(_) => return py.None(),
    };
    let entities = world.entities_in_cell(&cell_key);
    serde_pyobject::to_pyobject(py, &entities).unwrap().into()
}

/// Get metadata associated with a given cell.
///
/// `cell` is a Python object representing a cell key.
/// Returns the metadata as a Python object or `None` if no metadata found.
pub fn get_cell_metadata(pyworld: &PyWorld, py: Python, cell: &Bound<'_, PyAny>) -> PyObject {
    let world = pyworld.inner.borrow();
    let cell_key: engine_core::map::CellKey = match pythonize::depythonize(cell) {
        Ok(val) => val,
        Err(_) => return py.None(),
    };
    if let Some(meta) = world.get_cell_metadata(&cell_key) {
        serde_pyobject::to_pyobject(py, meta).unwrap().into()
    } else {
        py.None()
    }
}

/// Set metadata for a given cell.
///
/// `cell` and `metadata` are Python objects representing the cell key and the metadata respectively.
pub fn set_cell_metadata(
    pyworld: &PyWorld,
    cell: &Bound<'_, PyAny>,
    metadata: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    let cell_key: engine_core::map::CellKey = depythonize(cell)?;
    let meta_json: serde_json::Value = depythonize(metadata)?;
    world.set_cell_metadata(&cell_key, meta_json);
    Ok(())
}

/// Find a path between two cells using the map's pathfinding system.
///
/// `start` and `goal` are Python objects representing cell keys (e.g., coordinates).
///
/// Returns a Python dictionary with keys:
/// - `"path"`: list of cells representing the path
/// - `"total_cost"`: total cost of the path
///   If no path is found, returns Python None.
pub fn find_path(
    pyworld: &PyWorld,
    py: Python,
    start: &Bound<'_, PyAny>,
    goal: &Bound<'_, PyAny>,
) -> PyObject {
    let world = pyworld.inner.borrow();

    // Convert Python objects to CellKey Rust types
    let start_key: engine_core::map::CellKey = match pythonize::depythonize(start) {
        Ok(v) => v,
        Err(_) => return py.None(),
    };
    let goal_key: engine_core::map::CellKey = match pythonize::depythonize(goal) {
        Ok(v) => v,
        Err(_) => return py.None(),
    };

    if let Some(result) = world.find_path(&start_key, &goal_key) {
        let dict = PyDict::new(py);
        let _ = dict.set_item(
            "path",
            serde_pyobject::to_pyobject(py, &result.path).unwrap(),
        );
        let _ = dict.set_item("total_cost", result.total_cost);
        dict.into()
    } else {
        py.None()
    }
}

/// Register a Python callback as a map validator.
pub fn register_map_validator(pyworld: &PyWorld, py: Python, callback: Py<PyAny>) {
    pyworld
        .map_validators
        .borrow_mut()
        .push(callback.clone_ref(py));
}

/// Clear all registered Python map validators.
pub fn clear_map_validators(pyworld: &PyWorld) {
    pyworld.map_validators.borrow_mut().clear();
}

/// Register a Python callback as a map postprocessor.
pub fn register_map_postprocessor(pyworld: &PyWorld, py: Python, callback: Py<PyAny>) {
    pyworld
        .map_postprocessors
        .borrow_mut()
        .push(callback.clone_ref(py));
}

/// Clear all registered Python map postprocessors.
pub fn clear_map_postprocessors(pyworld: &PyWorld) {
    pyworld.map_postprocessors.borrow_mut().clear();
}

/// Apply a generated map JSON to the world.
///
/// Runs registered validators and postprocessors.
pub fn apply_generated_map(
    pyworld: Py<PyWorld>, // Python-owned PyWorld
    py: Python,
    map: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let slf = pyworld.borrow(py);

    let map_json: serde_json::Value = depythonize(map)?;

    // Validate using map validators
    let validators = slf.map_validators.borrow();
    for callback in validators.iter() {
        let ok: bool = callback.call1(py, (map.clone(),))?.extract(py)?;
        if !ok {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Map validator failed",
            ));
        }
    }

    // Apply the map
    {
        let mut world = slf.inner.borrow_mut();
        world
            .apply_generated_map(&map_json)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
    }

    // Call postprocessors
    let postprocessors = slf.map_postprocessors.borrow();
    for callback in postprocessors.iter() {
        callback.call1(py, (pyworld.clone_ref(py),))?;
    }

    Ok(())
}

/// Apply a chunk of map JSON data.
///
/// Used for incremental updates or streaming.
pub fn apply_chunk(pyworld: Py<PyWorld>, py: Python, chunk: &Bound<'_, PyAny>) -> PyResult<()> {
    let slf = pyworld.borrow(py);
    let chunk_json: serde_json::Value = depythonize(chunk)?;
    let mut world = slf.inner.borrow_mut();
    world
        .apply_chunk(&chunk_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

/// Add a cell to the map.
pub fn add_cell(pyworld: &PyWorld, x: i32, y: i32, z: i32) {
    let mut world = pyworld.inner.borrow_mut();
    if let Some(map) = &mut world.map
        && let Some(square) = map
            .topology
            .as_any_mut()
            .downcast_mut::<engine_core::map::SquareGridMap>()
        {
            square.add_cell(x, y, z);
        }
}
