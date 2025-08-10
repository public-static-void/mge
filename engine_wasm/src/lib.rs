//! WASM scripting engine bridge for Modular Game Engine.
//!
//! This crate is a bridge between the Modular Game Engine and WASM, providing a way to
//! run scripts written in WASM.

/// The engine
pub mod engine;
/// The host API
pub mod host_api;

pub use engine::{WasmScriptEngine, WasmScriptEngineConfig, WasmValue};
