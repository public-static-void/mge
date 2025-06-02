pub mod api;
mod event_bus;
pub mod python_api;
mod system_bridge;
mod worldgen_bridge;

use crate::python_api::UiApi;
use api::PyWorld;
use engine_core::presentation::ui::register_all_widgets;
use pyo3::prelude::*;
use worldgen_bridge::{invoke_worldgen_plugin, list_worldgen_plugins, register_worldgen_plugin};

#[pymodule]
fn mge(_py: Python, m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
    register_all_widgets();
    m.add_class::<PyWorld>()?;
    m.add_function(wrap_pyfunction!(register_worldgen_plugin, m)?)?;
    m.add_function(wrap_pyfunction!(list_worldgen_plugins, m)?)?;
    m.add_function(wrap_pyfunction!(invoke_worldgen_plugin, m)?)?;
    m.add_class::<UiApi>()?;
    Ok(())
}
