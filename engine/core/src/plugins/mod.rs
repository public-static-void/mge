//! Plugin system
//!
//! The plugin system allows for loading and running plugins.

/// Dynamic systems
pub mod dynamic_systems;
/// FFI
pub mod ffi;
/// Plugin loader
pub mod loader;
/// Plugin manager
pub mod manager;
/// Plugin registry
pub mod registry;
/// Plugin subprocess
pub mod subprocess;
/// Plugin types
pub mod types;

pub use dynamic_systems::*;
pub use ffi::*;
pub use loader::*;
pub use registry::*;
pub use subprocess::{PluginRequest, PluginResponse};
pub use types::*;
