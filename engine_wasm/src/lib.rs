//! WASM scripting engine bridge for Modular Game Engine.

pub mod engine;
pub mod host_api;

pub use engine::{WasmScriptEngine, WasmScriptEngineConfig, WasmValue};
