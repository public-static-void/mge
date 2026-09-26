//! WASM host API for crafting recipe registration, gating, and order control.
//!
//! Functions registered under the `"craft"` namespace:
//! - `register_craft_recipe(name_ptr, name_len, recipe_ptr, recipe_len, out_ptr, out_len) -> i32`
//! - `list_craft_recipes(out_ptr, out_len) -> i32`
//! - `can_craft(crafter, recipe_ptr, recipe_len, out_ptr, out_len) -> i32`
//! - `start_craft(crafter, recipe_ptr, recipe_len, out_ptr, out_len) -> i32`
//! - `get_craft_state(crafter, out_ptr, out_len) -> i32`
//! - `cancel_craft(crafter, out_ptr, out_len) -> i32`
//!
//! JSON strings are the bridge transport (matching the surrounding host
//! APIs), not a shape difference: the recipe definition is one JSON object,
//! the recipe list is a JSON array of names, and the craft state is a
//! JSON-encoded `CraftOrder` (`"null"` when absent). Gate results cannot
//! return a guest-side tuple, so they write an `{"ok": bool, "err":
//! string|null}` envelope into the out buffer and return the bytes written
//! (`-1` when the buffer is too small). Error strings are byte-identical to
//! the Rust/Lua/Python surfaces: `unknown_recipe`, `already_crafting`,
//! `missing_tool:<item>`, `missing_material:<material>`,
//! `missing_input:<kind>`, `insufficient_skill`, `no_craft_order`.
//!
//! Example (guest pseudocode):
//! ```text
//! call craft.register_craft_recipe(name_ptr, name_len, recipe_ptr, recipe_len, out_ptr, out_len)
//! # out holds {"ok": true, "err": null} on success
//! ```

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::json;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Writes a gate `(ok, err)` envelope into the out buffer.
/// Returns bytes written, or `-1` when the buffer is too small.
fn write_result_envelope<T>(
    caller: &mut Caller<'_, T>,
    out_ptr: i32,
    out_len: i32,
    result: Result<bool, String>,
) -> i32 {
    let payload = match result {
        Ok(done) => json!({"ok": done, "err": serde_json::Value::Null}),
        Err(err) => json!({"ok": false, "err": err}),
    }
    .to_string();
    if payload.len() > out_len as usize {
        return -1;
    }
    write_string_to_wasm(caller, out_ptr, out_len, &payload) as i32
}

/// Registers the crafting API (register_craft_recipe, list_craft_recipes,
/// can_craft, start_craft, get_craft_state, cancel_craft).
pub fn register_craft_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "craft",
        "register_craft_recipe",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         recipe_ptr: i32,
         recipe_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read recipe name from WASM memory");
            let recipe_json = read_wasm_string(&mut caller, recipe_ptr, recipe_len)
                .expect("Failed to read recipe JSON from WASM memory");
            let result = {
                let mut world = caller.data().lock().unwrap();
                world
                    .register_craft_recipe(&name, &recipe_json)
                    .map(|()| true)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    linker.func_wrap(
        "craft",
        "list_craft_recipes",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let payload = {
                let world = caller.data().lock().unwrap();
                json!(world.list_craft_recipes()).to_string()
            };
            if payload.len() > out_len as usize {
                return -1;
            }
            write_string_to_wasm(&mut caller, out_ptr, out_len, &payload) as i32
        },
    )?;

    linker.func_wrap(
        "craft",
        "can_craft",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         crafter: u32,
         recipe_ptr: i32,
         recipe_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let recipe = read_wasm_string(&mut caller, recipe_ptr, recipe_len)
                .expect("Failed to read recipe name from WASM memory");
            let result = {
                let world = caller.data().lock().unwrap();
                world.can_craft(crafter, &recipe).map(|()| true)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    linker.func_wrap(
        "craft",
        "start_craft",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         crafter: u32,
         recipe_ptr: i32,
         recipe_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let recipe = read_wasm_string(&mut caller, recipe_ptr, recipe_len)
                .expect("Failed to read recipe name from WASM memory");
            let result = {
                let mut world = caller.data().lock().unwrap();
                world.start_craft(crafter, &recipe).map(|()| true)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    linker.func_wrap(
        "craft",
        "get_craft_state",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         crafter: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let payload = {
                let world = caller.data().lock().unwrap();
                world
                    .get_craft_state(crafter)
                    .unwrap_or_else(|| "null".to_string())
            };
            if payload.len() > out_len as usize {
                return -1;
            }
            write_string_to_wasm(&mut caller, out_ptr, out_len, &payload) as i32
        },
    )?;

    linker.func_wrap(
        "craft",
        "cancel_craft",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         crafter: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let mut world = caller.data().lock().unwrap();
                world.cancel_craft(crafter)
            };
            write_result_envelope(&mut caller, out_ptr, out_len, result)
        },
    )?;

    Ok(())
}
