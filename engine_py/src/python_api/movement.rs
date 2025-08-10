// engine_py/src/python_api/movement.rs

use crate::python_api::world::PyWorld;
use engine_core::map::CellKey;
use engine_core::systems::job::ops::movement_ops;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pythonize::depythonize;

/// Adapter for exposed Python movement methods.
pub struct MovementApi;

impl MovementApi {
    /// Assign a move path to an agent from one cell to another.
    pub fn assign_move_path(
        pyworld: &PyWorld,
        agent_id: u32,
        from_cell: &Bound<'_, PyAny>,
        to_cell: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let from_val: serde_json::Value = depythonize(from_cell)
            .map_err(|e| PyValueError::new_err(format!("Invalid from_cell: {e}")))?;
        let to_val: serde_json::Value = depythonize(to_cell)
            .map_err(|e| PyValueError::new_err(format!("Invalid to_cell: {e}")))?;

        let from_key: CellKey = serde_json::from_value(from_val)
            .map_err(|e| PyValueError::new_err(format!("Invalid from_cell key: {e}")))?;
        let to_key: CellKey = serde_json::from_value(to_val)
            .map_err(|e| PyValueError::new_err(format!("Invalid to_cell key: {e}")))?;

        let mut world = pyworld.inner.borrow_mut();
        movement_ops::assign_move_path(&mut world, agent_id, &from_key, &to_key);
        Ok(())
    }

    /// Check if an agent is at the given cell.
    pub fn is_agent_at_cell(
        pyworld: &PyWorld,
        agent_id: u32,
        cell: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let val: serde_json::Value =
            depythonize(cell).map_err(|e| PyValueError::new_err(format!("Invalid cell: {e}")))?;
        let cell_key: CellKey = serde_json::from_value(val)
            .map_err(|e| PyValueError::new_err(format!("Invalid cell key: {e}")))?;

        let world = pyworld.inner.borrow();
        Ok(movement_ops::is_agent_at_cell(&world, agent_id, &cell_key))
    }

    /// Check if an agent's movement path is currently empty.
    pub fn is_move_path_empty(pyworld: &PyWorld, agent_id: u32) -> PyResult<bool> {
        let world = pyworld.inner.borrow();
        Ok(movement_ops::is_move_path_empty(&world, agent_id))
    }
}
