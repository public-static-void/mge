//! Building construction system.
//!
//! Implements the colony-sim construction loop as a Job-system extension:
//! a validated blueprint spawns a ghost entity carrying a `ConstructionSite`
//! component linked to a `Job(category=construction, job_type=construct)`.
//! Reservation, delivery states, and worker assignment are reused untouched
//! from the job layer; this system only mirrors delivery into site state,
//! consumes materials exactly once on delivery, ticks work progress, and
//! completes the site into a `Building`.
//!
//! Cancel/demolish semantics live in a later milestone; the bridge op
//! surface (Lua/Python/WASM) is also layered later. This module owns the
//! core loop only: [`place_blueprint`] plus [`ConstructionSystem`].

/// Construction progress and completion system.
pub mod system;

pub use system::{ConstructionSystem, cell_position_json, place_blueprint};
