//! Narrative WASM host API.
//!
//! All functions are registered on the "narrative" linker module.
//! Every transition delegates to `engine_core::narrative::NarrativeState`
//! (the same pure `apply_tick` / `apply_resolve` rules the `World` API
//! uses); this layer only moves JSON strings across the guest boundary and
//! forwards the resulting events. Payload shapes serialize from the same
//! core types, keeping the Lua/Python/WASM surfaces byte-compatible.

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the narrative API (register_scenario, list_scenarios,
/// get_scenario, poll_pending_decisions, get_pending_decision,
/// resolve_decision, get_narrative_history).
pub fn register_narrative_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    // register_scenario(def_ptr, def_len) -> 0 on success, -1 on error
    linker.func_wrap(
        "narrative",
        "register_scenario",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, def_ptr: i32, def_len: i32| -> i32 {
            let def_json = match read_wasm_string(&mut caller, def_ptr, def_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let parsed: Result<engine_core::narrative::ScenarioDef, _> =
                serde_json::from_str(&def_json);
            let mut world = caller.data().lock().unwrap();
            match parsed {
                Ok(def) => match world.narrative.register_scenario(def) {
                    Ok(()) => 0,
                    Err(_) => -1,
                },
                Err(_) => -1,
            }
        },
    )?;

    // list_scenarios(out_ptr, out_len) -> bytes written (id-sorted JSON array)
    linker.func_wrap(
        "narrative",
        "list_scenarios",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let data = {
                let world = caller.data().lock().unwrap();
                let mut defs: Vec<_> = world.narrative.scenarios.values().cloned().collect();
                defs.sort_by(|a, b| a.id.cmp(&b.id));
                serde_json::to_string(&defs).unwrap_or_else(|_| "[]".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    // get_scenario(id_ptr, id_len, out_ptr, out_len) -> bytes written, -1 if missing
    linker.func_wrap(
        "narrative",
        "get_scenario",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         id_ptr: i32,
         id_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let id = match read_wasm_string(&mut caller, id_ptr, id_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let data = {
                let world = caller.data().lock().unwrap();
                match world.narrative.scenarios.get(&id) {
                    Some(def) => serde_json::to_string(def).unwrap_or_else(|_| "{}".to_string()),
                    None => return -1,
                }
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    // poll_pending_decisions(out_ptr, out_len) -> bytes written (id-ordered JSON array)
    linker.func_wrap(
        "narrative",
        "poll_pending_decisions",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let data = {
                let world = caller.data().lock().unwrap();
                let mut pending: Vec<_> = world.narrative.pending.values().cloned().collect();
                pending.sort_by_key(|decision| decision.id);
                serde_json::to_string(&pending).unwrap_or_else(|_| "[]".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    // get_pending_decision(id, out_ptr, out_len) -> bytes written, -1 if missing
    linker.func_wrap(
        "narrative",
        "get_pending_decision",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         id: i64,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let decision_id = u64::try_from(id).unwrap_or(u64::MAX);
            let data = {
                let world = caller.data().lock().unwrap();
                match world.narrative.pending.get(&decision_id) {
                    Some(decision) => {
                        serde_json::to_string(decision).unwrap_or_else(|_| "{}".to_string())
                    }
                    None => return -1,
                }
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    // resolve_decision(id, choice_ptr, choice_len) -> 0 on success, -1 on error
    linker.func_wrap(
        "narrative",
        "resolve_decision",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         id: i64,
         choice_ptr: i32,
         choice_len: i32|
         -> i32 {
            let choice_id = match read_wasm_string(&mut caller, choice_ptr, choice_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let decision_id = u64::try_from(id).unwrap_or(u64::MAX);
            let mut world = caller.data().lock().unwrap();
            match world.resolve_narrative_decision(decision_id, &choice_id) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    // get_narrative_history(out_ptr, out_len) -> bytes written (append-order JSON array)
    linker.func_wrap(
        "narrative",
        "get_narrative_history",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let data = {
                let world = caller.data().lock().unwrap();
                serde_json::to_string(&world.narrative.history).unwrap_or_else(|_| "[]".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    Ok(())
}
