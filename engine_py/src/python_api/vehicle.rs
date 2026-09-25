use crate::python_api::world::PyWorld;
use engine_core::map::CellKey;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pythonize::depythonize;

/// Parses a goal [`CellKey`] from a Python cell dict (enum form first, then
/// pos-wrapped shapes via [`CellKey::from_position`).
fn parse_goal_cell(goal: &Bound<'_, PyAny>) -> PyResult<CellKey> {
    let val: serde_json::Value =
        depythonize(goal).map_err(|e| PyValueError::new_err(format!("Invalid goal_cell: {e}")))?;
    if let Ok(key) = serde_json::from_value::<CellKey>(val.clone()) {
        return Ok(key);
    }
    CellKey::from_position(&val)
        .ok_or_else(|| PyValueError::new_err("Invalid goal_cell key format"))
}

/// Embark a rider onto a vehicle.
///
/// Returns `(ok, err)` mirroring the Lua surface: `(True, None)` on success,
/// `(False, reason)` with one of `no_vehicle/no_rider/already_mounted/full`
/// on rejection.
pub fn embark_vehicle(pyworld: &PyWorld, vehicle_id: u32, rider_id: u32) -> (bool, Option<String>) {
    let mut world = pyworld.inner.borrow_mut();
    match world.embark(vehicle_id, rider_id) {
        Ok(()) => (true, None),
        Err(err) => (false, Some(err)),
    }
}

/// Disembark a rider from its vehicle.
///
/// Returns `(ok, err)`: `(True, None)` on success, `(False, "not_mounted")`
/// when the rider is not mounted.
pub fn disembark_vehicle(pyworld: &PyWorld, rider_id: u32) -> (bool, Option<String>) {
    let mut world = pyworld.inner.borrow_mut();
    match world.disembark(rider_id) {
        Ok(()) => (true, None),
        Err(err) => (false, Some(err)),
    }
}

/// Compute a path with the existing A* and store the terrain-valid prefix in
/// `Vehicle.move_path`, returning the stored step count.
///
/// Argument order mirrors the Lua/WASM surface: `(vehicle_id, goal_cell)`.
/// Raises `ValueError` carrying the `no_vehicle`/`no_path` reason on failure.
pub fn assign_vehicle_path(
    pyworld: &PyWorld,
    vehicle_id: u32,
    goal_cell: &Bound<'_, PyAny>,
) -> PyResult<usize> {
    let goal_key = parse_goal_cell(goal_cell)?;
    let mut world = pyworld.inner.borrow_mut();
    world
        .assign_vehicle_path(vehicle_id, &goal_key)
        .map_err(PyValueError::new_err)
}

/// Occupant entity IDs of a vehicle, or empty when it has no `Vehicle`
/// component.
pub fn get_vehicle_occupants(pyworld: &PyWorld, vehicle_id: u32) -> Vec<u32> {
    let world = pyworld.inner.borrow();
    world.vehicle_occupants(vehicle_id)
}

/// True when the rider is listed in any live vehicle's `occupants`.
pub fn is_mounted(pyworld: &PyWorld, rider_id: u32) -> bool {
    let world = pyworld.inner.borrow();
    world.is_mounted(rider_id)
}
