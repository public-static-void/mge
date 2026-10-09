//! Job event-logger init shim.
//!
//! Classification: pure transport — stays here, no logic moves into
//! `python_api/` domains. Single delegating call into the engine core logger
//! singleton; the `job_events` domain logic lives in
//! `python_api::job_events`.

use pyo3::prelude::*;

/// Initialize the Rust-side job event logger singleton.
/// This must be called before any job events are emitted from Python.
#[pyfunction]
pub fn py_init_job_event_logger() {
    engine_core::systems::job::system::events::init_job_event_logger();
}
