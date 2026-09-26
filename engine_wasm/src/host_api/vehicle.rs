//! WASM host API for vehicle carrier functions.
//!
//! Functions registered under the `"vehicle"` namespace:
//! - `embark_vehicle(vehicle, rider, out_ptr, out_len) -> i32`
//! - `disembark_vehicle(rider, out_ptr, out_len) -> i32`
//! - `assign_vehicle_path(vehicle, goal_ptr, goal_len) -> i32`
//! - `get_vehicle_occupants(vehicle, out_ptr, out_len) -> i32`
//! - `is_mounted(rider) -> i32`
//!
//! JSON strings are the bridge transport (matching the surrounding host
//! APIs), not a shape difference: the goal cell is one enum-shaped cell
//! (`{"Square": ...}` / `{"Hex": ...}` / `{"Province": ...}`), and the
//! occupant list is a JSON array of entity ids. Embark/disembark cannot
//! return a guest-side tuple, so they write an `{"ok": bool, "err":
//! string|null}` envelope into the out buffer and return the bytes written
//! (`-1` when the buffer is too small). `assign_vehicle_path` returns the
//! stored step count (`-1` on `no_vehicle`/`no_path`); `is_mounted` returns
//! `1`/`0`.

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::json;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Writes the embark/disembark `(ok, err)` envelope into the out buffer.
/// Returns bytes written, or `-1` when the buffer is too small.
fn write_result_envelope<T>(
    caller: &mut Caller<'_, T>,
    out_ptr: i32,
    out_len: i32,
    result: Result<(), String>,
) -> i32 {
    let payload = match result {
        Ok(()) => json!({"ok": true, "err": serde_json::Value::Null}),
        Err(err) => json!({"ok": false, "err": err}),
    }
    .to_string();
    if payload.len() > out_len as usize {
        return -1;
    }
    write_string_to_wasm(caller, out_ptr, out_len, &payload) as i32
}

/// Registers the vehicle API (embark_vehicle, disembark_vehicle,
/// assign_vehicle_path, get_vehicle_occupants, is_mounted).
pub fn register_vehicle_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "vehicle",
        "embark_vehicle",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         vehicle: u32,
         rider: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let mut world = caller.data().lock().unwrap();
                world.embark(vehicle, rider)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    linker.func_wrap(
        "vehicle",
        "disembark_vehicle",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         rider: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let mut world = caller.data().lock().unwrap();
                world.disembark(rider)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    linker.func_wrap(
        "vehicle",
        "assign_vehicle_path",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         vehicle: u32,
         goal_ptr: i32,
         goal_len: i32|
         -> i32 {
            let goal_json = read_wasm_string(&mut caller, goal_ptr, goal_len)
                .expect("Failed to read goal cell JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            match world.assign_vehicle_path(vehicle, &goal_json) {
                Ok(steps) => steps as i32,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "vehicle",
        "get_vehicle_occupants",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         vehicle: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let payload = {
                let world = caller.data().lock().unwrap();
                json!(world.vehicle_occupants(vehicle)).to_string()
            };
            if payload.len() > out_len as usize {
                return -1;
            }
            write_string_to_wasm(&mut caller, out_ptr, out_len, &payload) as i32
        },
    )?;

    linker.func_wrap(
        "vehicle",
        "is_mounted",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, rider: u32| -> i32 {
            let world = caller.data().lock().unwrap();
            if world.is_mounted(rider) { 1 } else { 0 }
        },
    )?;

    Ok(())
}
