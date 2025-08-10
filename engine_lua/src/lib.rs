//! Lua scripting bridge for Modular Game Engine.
//!
//! This module provides a way to access the game engine from within a Lua script.

/// The game engine.
pub mod engine;
/// Event bus.
pub mod event_bus;
/// Helper functions.
pub mod helpers;
/// Input handling.
pub mod input;
/// Lua API.
pub mod lua_api;
/// Schemas.
pub mod schemas;

pub use engine::ScriptEngine;
