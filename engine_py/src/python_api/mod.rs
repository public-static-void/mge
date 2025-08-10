//! Python API for the engine.
//!
//! This module contains the Python API for the engine, which is used to
//! create Python objects that can be used in Python scripts.

/// Body API
pub mod body;
/// Camera API
pub mod camera_api;
/// Component API
pub mod component;
/// Death/decay API
pub mod death_decay;
/// Economic API
pub mod economic;
/// Entity API
pub mod entity;
/// Equipment API
pub mod equipment;
/// Inventory API
pub mod inventory;
/// Job AI API
pub mod job_ai;
/// Job API
pub mod job_api;
/// Job board API
pub mod job_board;
/// Job children API
pub mod job_children;
/// Job dependencies API
pub mod job_dependencies;
/// Job events API
pub mod job_events;
/// Job production API
pub mod job_production;
/// Job query API
pub mod job_query;
/// Job reservation API
pub mod job_reservation;
/// Map API
pub mod map_api;
/// Game mode API
pub mod mode;
/// Movement API
pub mod movement;
/// Region API
pub mod region;
/// Save/Load API
pub mod save_load;
/// Time API
pub mod time_of_day;
/// Turn API
pub mod turn;
/// UI API
pub mod ui;
/// World API
pub mod world;

pub use ui::UiApi;
pub use world::PyWorld;
