use pyo3::prelude::*;

/// Initialize the Rust-side job event logger singleton.
/// This must be called before any job events are emitted from Python.
#[pyfunction]
pub fn py_init_job_event_logger() {
    engine_core::systems::job::system::events::init_job_event_logger();
}
