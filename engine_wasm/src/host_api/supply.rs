//! Supply WASM host API.
//!
//! Functions registered under the `"supply"` namespace:
//! - `create_supply_link(source, target, kind_ptr, kind_len, amount, capacity, out_ptr, out_len) -> i32`
//! - `remove_supply_link(link, out_ptr, out_len) -> i32`
//! - `list_supply_links(out_ptr, out_len) -> i32`
//! - `set_supply_link_active(link, active, out_ptr, out_len) -> i32`
//! - `get_supply_link(link, out_ptr, out_len) -> i32`
//!
//! Every transition delegates to the `WasmWorld` supply mirror (the same
//! validation, atomicity, epsilon discipline, and gate order the `World` API
//! uses); this layer only moves strings across the guest boundary. JSON
//! strings are the bridge transport (matching the surrounding host APIs), not
//! a shape difference: `create`/`remove`/`set_active` write an
//! `{"ok": bool, "err": string|null}` envelope into the out buffer (plus the
//! `link` id on create), `get` writes `{"ok", "link": record|null, "err"}`,
//! and `list` writes a JSON array of ascending link ids. All functions return
//! the bytes written (`-1` when the buffer is too small). Error strings prefix
//! the core variant name (`SameEndpoint`, `UnknownKind`, `NonPositiveAmount`,
//! `NoStockpile`, `UnknownLink`, `Store`) so guest assertions match on it,
//! exactly like the Lua/Python bridges.

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::json;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Writes the mutation `(ok, err)` envelope into the out buffer.
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

/// Registers the supply API (create/remove/list/set_active/get supply link).
pub fn register_supply_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "supply",
        "create_supply_link",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         source: u32,
         target: u32,
         kind_ptr: i32,
         kind_len: i32,
         amount: f64,
         capacity: f64,
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
                world.create_supply_link(source, target, &kind, amount, capacity)
            };
            let payload = match result {
                Ok(link) => json!({"ok": true, "link": link, "err": serde_json::Value::Null}),
                Err(err) => json!({"ok": false, "link": serde_json::Value::Null, "err": err}),
            }
            .to_string();
            if payload.len() > out_len as usize {
                return -1;
            }
            write_string_to_wasm(&mut caller, out_ptr, out_len, &payload) as i32
        },
    )?;

    linker.func_wrap(
        "supply",
        "remove_supply_link",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         link: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let mut world = caller.data().lock().unwrap();
                world.remove_supply_link(link)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    linker.func_wrap(
        "supply",
        "list_supply_links",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let links = {
                let world = caller.data().lock().unwrap();
                world.list_supply_links()
            };
            let data = serde_json::to_string(&links).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    linker.func_wrap(
        "supply",
        "set_supply_link_active",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         link: u32,
         active: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let mut world = caller.data().lock().unwrap();
                world.set_supply_link_active(link, active != 0)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    linker.func_wrap(
        "supply",
        "get_supply_link",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         link: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let record = {
                let world = caller.data().lock().unwrap();
                world.get_supply_link(link).map(|route| {
                    json!({
                        "source": route.source,
                        "target": route.target,
                        "kind": route.kind,
                        "amount_per_tick": route.amount_per_tick,
                        "capacity_per_tick": route.capacity_per_tick,
                        "active": route.active,
                    })
                })
            };
            let payload = match record {
                Some(value) => json!({"ok": true, "link": value, "err": serde_json::Value::Null}),
                None => json!({"ok": false, "link": serde_json::Value::Null,
                    "err": format!("UnknownLink: entity {link} has no SupplyLink component")}),
            }
            .to_string();
            if payload.len() > out_len as usize {
                return -1;
            }
            write_string_to_wasm(&mut caller, out_ptr, out_len, &payload) as i32
        },
    )?;

    Ok(())
}
