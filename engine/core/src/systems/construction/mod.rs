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
//! Cancel/demolish semantics are part of this module: [`cancel_construction`]
//! refunds delivered materials and retires the ghost plus its job, while
//! [`demolish_building`] removes completed `Building` entities with no refund.
//! The bridge op surface (Lua/Python/WASM) calls into these core functions
//! with identical argument order and return shapes; each bridge enforces the
//! colony-mode gate before delegating. This module owns
//! the core loop plus teardown: [`place_blueprint`], [`get_construction_state`],
//! [`cancel_construction`], [`demolish_building`], and [`ConstructionSystem`].

/// Construction progress and completion system.
pub mod system;

pub use system::{
    ConstructionSystem, cancel_construction, cell_position_json, demolish_building,
    get_construction_state, place_blueprint,
};
