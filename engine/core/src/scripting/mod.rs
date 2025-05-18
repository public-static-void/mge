//! # Lua Scripting Bridge
//!
//! ## Exposed Functions
//! - `spawn_entity()` -> entity id
//! - `set_position(entity, x, y)`
//! - `get_position(entity)` -> {x, y} or nil
//! - `set_health(entity, current, max)`
//! - `get_health(entity)` -> {current, max} or nil
//!
//! ## Adding More Components
//! 1. Extend `World` with your component storage.
//! 2. Add set/get methods.
//! 3. Register new Lua functions in `register_world`.
//! 4. Add Lua and Rust tests.

pub mod input;

pub mod world;
pub use world::World;

pub mod engine;
pub use engine::ScriptEngine;

pub mod api;
pub mod event_bus;
pub mod helpers;
pub mod system_bridge;
pub mod worldgen_bridge;
