mod api;
mod event_bus;
mod system_bridge;
mod worldgen_bridge;

use api::PyWorld;
use pyo3::prelude::*;

#[pymodule]
fn mge(_py: Python, m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
    m.add_class::<PyWorld>()?;
    Ok(())
}
