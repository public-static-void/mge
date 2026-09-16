use crate::PyObject;
use crate::python_api::world::PyWorld;
use engine_core::map::CellKey;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pythonize::depythonize;

/// Parses a [`CellKey`] from a Python cell dict (enum form first, then
/// pos-wrapped shapes via [`CellKey::from_position`]).
fn parse_cell(cell: &Bound<'_, PyAny>) -> PyResult<CellKey> {
    let val: serde_json::Value =
        depythonize(cell).map_err(|e| PyValueError::new_err(format!("Invalid cell: {e}")))?;
    if let Ok(key) = serde_json::from_value::<CellKey>(val.clone()) {
        return Ok(key);
    }
    CellKey::from_position(&val).ok_or_else(|| PyValueError::new_err("Invalid cell key format"))
}

/// Parses `required_materials` (list of `{kind, amount}`) into integer pairs.
fn parse_materials(materials: &Bound<'_, PyAny>) -> PyResult<Vec<(String, i64)>> {
    let val: serde_json::Value = depythonize(materials)
        .map_err(|e| PyValueError::new_err(format!("Invalid required_materials: {e}")))?;
    let arr = val
        .as_array()
        .ok_or_else(|| PyValueError::new_err("required_materials must be an array"))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let kind = item
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PyValueError::new_err("material entry missing string 'kind'"))?
            .to_string();
        let amount = item
            .get("amount")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
            })
            .ok_or_else(|| PyValueError::new_err("material entry missing integer 'amount'"))?;
        out.push((kind, amount));
    }
    Ok(out)
}

/// Rejects non-colony callers with a mode-gated error before delegating.
fn require_colony(world: &engine_core::ecs::world::World, op: &str) -> PyResult<()> {
    if world.get_mode() != "colony" {
        return Err(PyValueError::new_err(format!(
            "{op}: world is in '{}' mode; construction requires 'colony' mode",
            world.get_mode()
        )));
    }
    Ok(())
}

/// Places a validated blueprint and returns the site entity id.
///
/// Argument order mirrors the Lua/WASM surface:
/// `(building_type, cell, required_materials, required_work)`.
pub fn place_blueprint(
    pyworld: &PyWorld,
    building_type: String,
    cell: &Bound<'_, PyAny>,
    required_materials: &Bound<'_, PyAny>,
    required_work: i64,
) -> PyResult<u32> {
    let cell_key = parse_cell(cell)?;
    let materials = parse_materials(required_materials)?;
    let mut world = pyworld.inner.borrow_mut();
    require_colony(&world, "place_blueprint")?;
    engine_core::systems::construction::place_blueprint(
        &mut world,
        &building_type,
        &cell_key,
        &materials,
        required_work,
    )
    .map_err(PyValueError::new_err)
}

/// Returns `{ state, progress, required_work, building_type }` for a site.
pub fn get_construction_state(pyworld: &PyWorld, py: Python, site_id: u32) -> PyResult<PyObject> {
    let world = pyworld.inner.borrow();
    require_colony(&world, "get_construction_state")?;
    let state = engine_core::systems::construction::get_construction_state(&world, site_id)
        .map_err(PyValueError::new_err)?;
    Ok(serde_pyobject::to_pyobject(py, &state)?.into())
}

/// Cancels a pre-completion site; returns True.
pub fn cancel_construction(pyworld: &PyWorld, site_id: u32) -> PyResult<bool> {
    let mut world = pyworld.inner.borrow_mut();
    require_colony(&world, "cancel_construction")?;
    engine_core::systems::construction::cancel_construction(&mut world, site_id)
        .map_err(PyValueError::new_err)
}

/// Demolishes a completed building; returns True.
pub fn demolish_building(pyworld: &PyWorld, building_id: u32) -> PyResult<bool> {
    let mut world = pyworld.inner.borrow_mut();
    require_colony(&world, "demolish_building")?;
    engine_core::systems::construction::demolish_building(&mut world, building_id)
        .map_err(PyValueError::new_err)
}
