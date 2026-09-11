//! WASM host API for noise and detection functions.
//!
//! Functions registered under the `"wasm_noise"` namespace:
//! - `emit_noise(entity, intensity, radius) -> i32`
//! - `get_noise_at(x, y, z) -> f64`
//! - `set_hearing(entity, range, sensitivity) -> i32`
//! - `get_hearing(entity, out_ptr, out_len) -> i32`

use crate::host_api::component::write_string_to_wasm;
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::map::CellKey;
use serde_json::json;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the noise API (emit_noise, get_noise_at, set_hearing, get_hearing).
pub fn register_noise_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "wasm_noise",
        "emit_noise",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         intensity: f64,
         radius: i32|
         -> i32 {
            let mut world = caller.data().lock().unwrap();
            let data = json!({
                "intensity": intensity,
                "radius": radius,
                "active": true,
            });
            match world.set_component(entity, "NoiseEmitter", &data.to_string()) {
                Ok(()) => 1,
                Err(_) => 0,
            }
        },
    )?;

    linker.func_wrap(
        "wasm_noise",
        "get_noise_at",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, x: i32, y: i32, z: i32| -> f64 {
            let world = caller.data().lock().unwrap();
            let cell = CellKey::Square { x, y, z };
            world.get_noise_at(&cell).unwrap_or(0.0)
        },
    )?;

    linker.func_wrap(
        "wasm_noise",
        "set_hearing",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         range: i32,
         sensitivity: f64|
         -> i32 {
            let mut world = caller.data().lock().unwrap();
            let data = json!({
                "range": range,
                "sensitivity": sensitivity,
            });
            match world.set_component(entity, "Hearing", &data.to_string()) {
                Ok(()) => 1,
                Err(_) => 0,
            }
        },
    )?;

    linker.func_wrap(
        "wasm_noise",
        "get_hearing",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_component(entity, "Hearing")
            };
            match result {
                Some(json_str) => {
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
                }
                None => -1,
            }
        },
    )?;

    Ok(())
}
