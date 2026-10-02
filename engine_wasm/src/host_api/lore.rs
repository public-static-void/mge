//! Lore WASM host API.
//!
//! All functions are registered on the "lore" linker module.
//! Every transition delegates to the shared state-level cores in
//! `engine_core::lore` (the same backfill, query, and rendering rules the
//! `World` API uses); this layer only moves JSON strings across the guest
//! boundary. Payload shapes serialize from the same core types, keeping the
//! Lua/Python/WASM surfaces byte-compatible.

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::lore::{ChronicleFilter, ChronicleKind};
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Parses a guest filter object into its core filter.
///
/// Missing keys stay unbounded. Rejects anything outside the fixed
/// `fired|resolved|expired|founding` kind set so invalid filters error
/// instead of silently matching nothing, matching the Lua/Python bridges.
fn parse_filter_object(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<ChronicleFilter, String> {
    let mut out = ChronicleFilter::default();
    if let Some(scenario) = obj.get("scenario_id") {
        if !scenario.is_null() {
            out.scenario_id = Some(
                scenario
                    .as_str()
                    .ok_or_else(|| "Chronicle filter scenario_id must be a string".to_string())?
                    .to_string(),
            );
        }
    }
    if let Some(kind) = obj.get("kind") {
        if !kind.is_null() {
            let raw = kind
                .as_str()
                .ok_or_else(|| "Chronicle filter kind must be a string".to_string())?;
            out.kind = Some(
                ChronicleKind::parse(raw)
                    .ok_or_else(|| format!("Invalid chronicle kind: {raw}"))?,
            );
        }
    }
    if let Some(from) = obj.get("turn_from") {
        if !from.is_null() {
            out.turn_from = Some(from.as_u64().ok_or_else(|| {
                "Chronicle filter turn_from must be a non-negative integer".to_string()
            })?);
        }
    }
    if let Some(to) = obj.get("turn_to") {
        if !to.is_null() {
            out.turn_to = Some(to.as_u64().ok_or_else(|| {
                "Chronicle filter turn_to must be a non-negative integer".to_string()
            })?);
        }
    }
    Ok(out)
}

/// Reads the guest filter JSON into its core filter.
///
/// A zero-length filter means unbounded. Malformed JSON and invalid kind
/// strings are errors; unknown entry ids are never errors (they read as
/// null on the single-lookup path).
fn read_filter(
    caller: &mut Caller<'_, Arc<Mutex<WasmWorld>>>,
    filter_ptr: i32,
    filter_len: i32,
) -> Result<ChronicleFilter, String> {
    if filter_len == 0 {
        return Ok(ChronicleFilter::default());
    }
    let raw = read_wasm_string(caller, filter_ptr, filter_len).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        return Ok(ChronicleFilter::default());
    }
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid chronicle filter: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "Chronicle filter must be a JSON object".to_string())?;
    parse_filter_object(obj)
}

/// Registers the lore API (generate_founding_history, list_chronicle,
/// get_chronicle_entry, render_chronicle, chronicle_len,
/// clear_lore_history).
pub fn register_lore_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    // generate_founding_history(seed, era_count) -> appended count, -1 on error
    linker.func_wrap(
        "lore",
        "generate_founding_history",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, seed: i64, era_count: i32| -> i64 {
            let seed_value = match u64::try_from(seed) {
                Ok(seed) => seed,
                Err(_) => return -1,
            };
            let era_value = match u32::try_from(era_count) {
                Ok(count) => count,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            match world.generate_founding_history(seed_value, era_value) {
                Ok(count) => count as i64,
                Err(_) => -1,
            }
        },
    )?;

    // list_chronicle(filter_ptr, filter_len, out_ptr, out_len) -> bytes written, -1 on error
    linker.func_wrap(
        "lore",
        "list_chronicle",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         filter_ptr: i32,
         filter_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let filter = match read_filter(&mut caller, filter_ptr, filter_len) {
                Ok(filter) => filter,
                Err(_) => return -1,
            };
            let data = {
                let world = caller.data().lock().unwrap();
                let entries = world.list_chronicle(filter);
                serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    // get_chronicle_entry(id, out_ptr, out_len) -> bytes written ("null" for unknown ids)
    linker.func_wrap(
        "lore",
        "get_chronicle_entry",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         id: i64,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let entry_id = u64::try_from(id).unwrap_or(u64::MAX);
            let data = {
                let world = caller.data().lock().unwrap();
                match world.get_chronicle_entry(entry_id) {
                    Some(entry) => {
                        serde_json::to_string(&entry).unwrap_or_else(|_| "null".to_string())
                    }
                    None => "null".to_string(),
                }
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    // render_chronicle(filter_ptr, filter_len, out_ptr, out_len) -> bytes written, -1 on error
    linker.func_wrap(
        "lore",
        "render_chronicle",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         filter_ptr: i32,
         filter_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let filter = match read_filter(&mut caller, filter_ptr, filter_len) {
                Ok(filter) => filter,
                Err(_) => return -1,
            };
            let data = {
                let world = caller.data().lock().unwrap();
                let lines = world.render_chronicle(filter);
                serde_json::to_string(&lines).unwrap_or_else(|_| "[]".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    // chronicle_len() -> number of chronicle entries
    linker.func_wrap(
        "lore",
        "chronicle_len",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> i64 {
            let world = caller.data().lock().unwrap();
            world.chronicle_len() as i64
        },
    )?;

    // clear_lore_history() -> 0; regen support before seeded backfill reruns
    linker.func_wrap(
        "lore",
        "clear_lore_history",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> i32 {
            let mut world = caller.data().lock().unwrap();
            world.clear_lore_history();
            0
        },
    )?;

    Ok(())
}
