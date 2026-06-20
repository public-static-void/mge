//! Host API
//!
//! This module contains the host API for the engine.

/// Entity module
pub mod entity;

/// Component module
pub mod component;

/// Turn module
pub mod turn;

/// Mode module
pub mod mode;

/// Death/Decay module
pub mod death_decay;

/// Time of day module
pub mod time_of_day;

/// UI module
pub mod ui;

/// UI tree module
pub mod ui_tree;

/// UI events module
pub mod ui_events;

/// Input module
pub mod input;

/// Inventory module
pub mod inventory;

/// Save/Load module
pub mod save_load;

/// Camera module
pub mod camera;

/// Event bus module
pub mod event_bus;

/// System module
pub mod system;

/// Movement operations module
pub mod movement_ops;

/// Equipment module
pub mod equipment;

/// Region module
pub mod region;

/// Body module
pub mod body;

/// Economic module
pub mod economic;

/// Map module
pub mod map;

/// Worldgen module
pub mod worldgen;

/// World userdata module (map chunk API, validators, postprocessors)
pub mod world_userdata;

/// Job system module (assign_job, get_job_types, register_job_type, get_job_type_metadata)
pub mod job_system;

/// Job board module (get/set job board, priorities)
pub mod job_board;

/// Job query module (list, find, get jobs, children, dependencies)
pub mod job_query;

/// Job mutation module (set_job_field, update_job)
pub mod job_mutation;

/// Job cancel module (cancel_job)
pub mod job_cancel;

/// Job events module (get_log, get_by_type, get_since, poll_bus, clear)
pub mod job_events;

/// Job AI module (ai_assign_jobs, ai_query_jobs, ai_modify_job_assignment)
pub mod job_ai;
