use crate::PyWorld;
use engine_core::map::CellKey;
use engine_core::systems::job::ops::movement_ops;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde_json::Value;

pub trait MovementApi {
    fn assign_move_path(&self, agent_id: u32, from_cell: Value, to_cell: Value) -> PyResult<()>;
    fn is_agent_at_cell(&self, agent_id: u32, cell: Value) -> PyResult<bool>;
    fn is_move_path_empty(&self, agent_id: u32) -> PyResult<bool>;
}

impl MovementApi for PyWorld {
    fn assign_move_path(&self, agent_id: u32, from_cell: Value, to_cell: Value) -> PyResult<()> {
        let from_cell_key: CellKey = serde_json::from_value(from_cell)
            .map_err(|e| PyValueError::new_err(format!("Invalid from_cell: {e}")))?;
        let to_cell_key: CellKey = serde_json::from_value(to_cell)
            .map_err(|e| PyValueError::new_err(format!("Invalid to_cell: {e}")))?;

        let mut world = self.inner.borrow_mut();

        // assign_move_path returns (), so don't call map_err here
        movement_ops::assign_move_path(&mut world, agent_id, &from_cell_key, &to_cell_key);

        Ok(())
    }

    fn is_agent_at_cell(&self, agent_id: u32, cell: Value) -> PyResult<bool> {
        let cell_key: CellKey = serde_json::from_value(cell)
            .map_err(|e| PyValueError::new_err(format!("Invalid cell: {e}")))?;

        let world = self.inner.borrow();

        Ok(movement_ops::is_agent_at_cell(&world, agent_id, &cell_key))
    }

    fn is_move_path_empty(&self, agent_id: u32) -> PyResult<bool> {
        let world = self.inner.borrow();

        Ok(movement_ops::is_move_path_empty(&world, agent_id))
    }
}
