//! Dungeon generation host API for WASM.
//!
//! Registers `generate_dungeon` under the `"dungeon"` namespace.
//! Accepts JSON config via pointer+length and returns JSON result.

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::systems::dungeon::{DungeonConfig, DungeonGenerator};
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the dungeon generation API on the wasmtime linker.
pub fn register_dungeon_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "dungeon",
        "generate_dungeon",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         config_ptr: i32,
         config_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            // Read config JSON from WASM memory
            let config_str = match read_wasm_string(&mut caller, config_ptr, config_len) {
                Ok(s) => s,
                Err(e) => {
                    let err = format!("Failed to read config: {e}");
                    return write_string_to_wasm(&mut caller, out_ptr, out_len, &err) as i32;
                }
            };

            // Parse config JSON
            let config_value: serde_json::Value = match serde_json::from_str(&config_str) {
                Ok(v) => v,
                Err(e) => {
                    let err = format!("Invalid config JSON: {e}");
                    return write_string_to_wasm(&mut caller, out_ptr, out_len, &err) as i32;
                }
            };

            // Build DungeonConfig from JSON (all fields optional)
            let mut cfg = DungeonConfig::default();
            if let Some(w) = config_value.get("width").and_then(|v| v.as_u64()) {
                if w == 0 {
                    let err = "Width must be positive".to_string();
                    return write_string_to_wasm(&mut caller, out_ptr, out_len, &err) as i32;
                }
                cfg.width = w as u32;
            }
            if let Some(h) = config_value.get("height").and_then(|v| v.as_u64()) {
                if h == 0 {
                    let err = "Height must be positive".to_string();
                    return write_string_to_wasm(&mut caller, out_ptr, out_len, &err) as i32;
                }
                cfg.height = h as u32;
            }
            if let Some(s) = config_value.get("seed").and_then(|v| v.as_u64()) {
                cfg.seed = s;
            }
            if let Some(v) = config_value.get("min_room_size").and_then(|v| v.as_u64()) {
                cfg.min_room_size = v as u32;
            }
            if let Some(v) = config_value.get("max_room_size").and_then(|v| v.as_u64()) {
                cfg.max_room_size = v as u32;
            }
            if let Some(v) = config_value.get("max_rooms").and_then(|v| v.as_u64()) {
                cfg.max_rooms = v as u32;
            }

            // Generate dungeon map
            match DungeonGenerator::generate(&cfg) {
                Ok(map) => {
                    let json_value = map.to_worldgen_json();
                    let json_str =
                        serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string());
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
                }
                Err(e) => {
                    let err = e.to_string();
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &err) as i32
                }
            }
        },
    )?;

    Ok(())
}
