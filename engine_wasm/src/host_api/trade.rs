//! Trade WASM host API.
//!
//! Functions registered under the `"trade"` namespace:
//! - `transfer_stockpile_resource(from, to, kind_ptr, kind_len, amount, out_ptr, out_len) -> i32`
//! - `has_active_trade_treaty(fa_ptr, fa_len, fb_ptr, fb_len) -> i32`
//! - `execute_treaty_trade(from, to, kind_ptr, kind_len, amount, out_ptr, out_len) -> i32`
//!
//! Every transition delegates to the `WasmWorld` trade mirror (the same
//! validation, atomicity, epsilon discipline, and gate order the `World` API
//! uses); this layer only moves strings across the guest boundary. JSON
//! strings are the bridge transport (matching the surrounding host APIs), not
//! a shape difference: transfers cannot return a guest-side tuple, so they
//! write an `{"ok": bool, "err": string|null}` envelope into the out buffer
//! and return the bytes written (`-1` when the buffer is too small). Error
//! strings prefix the core variant name (`NoStockpile`, `UnknownKind`,
//! `NonPositiveAmount`, `InsufficientFunds`, `NoLiveTreaty`, `RelationIsWar`)
//! so guest assertions match on it, exactly like the Lua/Python bridges.

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::json;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Writes the transfer `(ok, err)` envelope into the out buffer.
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

/// Reads a faction pair from guest memory.
fn read_pair(
    caller: &mut Caller<'_, Arc<Mutex<WasmWorld>>>,
    fa_ptr: i32,
    fa_len: i32,
    fb_ptr: i32,
    fb_len: i32,
) -> anyhow::Result<(String, String)> {
    Ok((
        read_wasm_string(caller, fa_ptr, fa_len)?,
        read_wasm_string(caller, fb_ptr, fb_len)?,
    ))
}

/// Registers the trade API (transfer_stockpile_resource,
/// has_active_trade_treaty, execute_treaty_trade).
pub fn register_trade_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "trade",
        "transfer_stockpile_resource",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         from: u32,
         to: u32,
         kind_ptr: i32,
         kind_len: i32,
         amount: f64,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let kind = match read_wasm_string(&mut caller, kind_ptr, kind_len) {
                Ok(kind) => kind,
                Err(e) => {
                    return write_result_envelope(
                        &mut caller,
                        out_ptr,
                        out_len,
                        Err(e.to_string()),
                    );
                }
            };
            let result = {
                let mut world = caller.data().lock().unwrap();
                world.transfer_stockpile_resource(from, to, &kind, amount)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    linker.func_wrap(
        "trade",
        "has_active_trade_treaty",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         fa_ptr: i32,
         fa_len: i32,
         fb_ptr: i32,
         fb_len: i32|
         -> i32 {
            let (fa, fb) = match read_pair(&mut caller, fa_ptr, fa_len, fb_ptr, fb_len) {
                Ok(pair) => pair,
                Err(_) => return 0,
            };
            let world = caller.data().lock().unwrap();
            if world.has_active_trade_treaty(&fa, &fb) {
                1
            } else {
                0
            }
        },
    )?;

    linker.func_wrap(
        "trade",
        "execute_treaty_trade",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         from: u32,
         to: u32,
         kind_ptr: i32,
         kind_len: i32,
         amount: f64,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let kind = match read_wasm_string(&mut caller, kind_ptr, kind_len) {
                Ok(kind) => kind,
                Err(e) => {
                    return write_result_envelope(
                        &mut caller,
                        out_ptr,
                        out_len,
                        Err(e.to_string()),
                    );
                }
            };
            let result = {
                let mut world = caller.data().lock().unwrap();
                world.execute_treaty_trade(from, to, &kind, amount)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    Ok(())
}
