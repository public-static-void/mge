//! Diplomacy WASM host API.
//!
//! All functions are registered on the "diplomacy" linker module.
//! Every transition delegates to `engine_core::diplomacy::DiplomacyState`
//! (the same logic the `World` API uses); this layer only moves strings
//! across the guest boundary and forwards the resulting events.

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::diplomacy::TreatyKind;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Forwards core-produced diplomacy events to the WASM event bus.
fn forward_pending(
    world: &mut WasmWorld,
    pending: Vec<(String, serde_json::Value)>,
) -> Result<(), String> {
    for (name, payload) in pending {
        let data = serde_json::to_string(&payload).unwrap_or_default();
        world.send_event(&name, &data)?;
    }
    Ok(())
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

/// Registers the diplomacy API (get_relation, get_standing, modify_standing,
/// declare_war, declare_peace, propose_treaty, accept_treaty, break_treaty,
/// list_treaties).
pub fn register_diplomacy_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "diplomacy",
        "get_relation",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         fa_ptr: i32,
         fa_len: i32,
         fb_ptr: i32,
         fb_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let (fa, fb) = match read_pair(&mut caller, fa_ptr, fa_len, fb_ptr, fb_len) {
                Ok(pair) => pair,
                Err(_) => return -1,
            };
            let relation = {
                let world = caller.data().lock().unwrap();
                world
                    .diplomacy
                    .query_relation(&fa, &fb)
                    .as_str()
                    .to_string()
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &relation) as i32
        },
    )?;

    linker.func_wrap(
        "diplomacy",
        "get_standing",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         fa_ptr: i32,
         fa_len: i32,
         fb_ptr: i32,
         fb_len: i32|
         -> i64 {
            let (fa, fb) = match read_pair(&mut caller, fa_ptr, fa_len, fb_ptr, fb_len) {
                Ok(pair) => pair,
                Err(_) => return 0,
            };
            let world = caller.data().lock().unwrap();
            world.diplomacy.query_standing(&fa, &fb)
        },
    )?;

    linker.func_wrap(
        "diplomacy",
        "modify_standing",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         fa_ptr: i32,
         fa_len: i32,
         fb_ptr: i32,
         fb_len: i32,
         delta: i64|
         -> i32 {
            let (fa, fb) = match read_pair(&mut caller, fa_ptr, fa_len, fb_ptr, fb_len) {
                Ok(pair) => pair,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            match world.diplomacy.apply_modify_standing(&fa, &fb, delta) {
                Ok(pending) => {
                    if forward_pending(&mut world, pending).is_err() {
                        return -1;
                    }
                    0
                }
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "diplomacy",
        "declare_war",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         fa_ptr: i32,
         fa_len: i32,
         fb_ptr: i32,
         fb_len: i32|
         -> i32 {
            let (fa, fb) = match read_pair(&mut caller, fa_ptr, fa_len, fb_ptr, fb_len) {
                Ok(pair) => pair,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            let turn = u64::from(world.turn);
            match world.diplomacy.apply_declare_war(&fa, &fb, turn) {
                Ok(pending) => {
                    if forward_pending(&mut world, pending).is_err() {
                        return -1;
                    }
                    0
                }
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "diplomacy",
        "declare_peace",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         fa_ptr: i32,
         fa_len: i32,
         fb_ptr: i32,
         fb_len: i32|
         -> i32 {
            let (fa, fb) = match read_pair(&mut caller, fa_ptr, fa_len, fb_ptr, fb_len) {
                Ok(pair) => pair,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            let turn = u64::from(world.turn);
            match world.diplomacy.apply_declare_peace(&fa, &fb, turn) {
                Ok(pending) => {
                    if forward_pending(&mut world, pending).is_err() {
                        return -1;
                    }
                    0
                }
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "diplomacy",
        "propose_treaty",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         proposer_ptr: i32,
         proposer_len: i32,
         other_ptr: i32,
         other_len: i32,
         kind_ptr: i32,
         kind_len: i32,
         duration: i64|
         -> i64 {
            let proposer = match read_wasm_string(&mut caller, proposer_ptr, proposer_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let other = match read_wasm_string(&mut caller, other_ptr, other_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let kind_raw = match read_wasm_string(&mut caller, kind_ptr, kind_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let kind = match TreatyKind::parse(&kind_raw) {
                Ok(kind) => kind,
                Err(_) => return -1,
            };
            let duration_ticks = u64::try_from(duration).ok();
            let mut world = caller.data().lock().unwrap();
            let turn = u64::from(world.turn);
            match world.diplomacy.apply_propose_treaty(
                &proposer,
                &other,
                kind,
                turn,
                duration_ticks,
            ) {
                Ok((id, pending)) => {
                    if forward_pending(&mut world, pending).is_err() {
                        return -1;
                    }
                    id as i64
                }
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "diplomacy",
        "accept_treaty",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, treaty_id: i64| -> i32 {
            let id = u64::try_from(treaty_id).unwrap_or(u64::MAX);
            let mut world = caller.data().lock().unwrap();
            let turn = u64::from(world.turn);
            match world.diplomacy.apply_accept_treaty(id, turn) {
                Ok(pending) => {
                    if forward_pending(&mut world, pending).is_err() {
                        return -1;
                    }
                    0
                }
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "diplomacy",
        "break_treaty",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, treaty_id: i64| -> i32 {
            let id = u64::try_from(treaty_id).unwrap_or(u64::MAX);
            let mut world = caller.data().lock().unwrap();
            match world.diplomacy.apply_break_treaty(id) {
                Ok(pending) => {
                    if forward_pending(&mut world, pending).is_err() {
                        return -1;
                    }
                    0
                }
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "diplomacy",
        "list_treaties",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         faction_ptr: i32,
         faction_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let faction = if faction_len == 0 {
                None
            } else {
                match read_wasm_string(&mut caller, faction_ptr, faction_len) {
                    Ok(s) => Some(s),
                    Err(_) => return -1,
                }
            };
            let data = {
                let world = caller.data().lock().unwrap();
                let records: Vec<serde_json::Value> = world
                    .diplomacy
                    .query_treaties(faction.as_deref())
                    .iter()
                    .map(|treaty| treaty.to_json())
                    .collect();
                serde_json::to_string(&serde_json::Value::Array(records))
                    .unwrap_or_else(|_| "[]".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32
        },
    )?;

    Ok(())
}
