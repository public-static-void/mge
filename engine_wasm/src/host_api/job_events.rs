use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the job events API (6 functions under the `"job_events"` import module).
///
/// Functions:
/// - `get_log(out_ptr, out_len)` — returns entire job event log as JSON array
/// - `get_by_type(type_ptr, type_len, out_ptr, out_len)` — returns events filtered by type
/// - `get_since(tick, out_ptr, out_len)` — returns events with timestamp >= tick
/// - `poll_bus(entity_id, type_ptr, type_len, out_ptr, out_len)` — takes events from event bus
/// - `clear()` — clears the entire job event log
/// - `get_where(filter_ptr, filter_len, out_ptr, out_len)` — returns events matching JSON filter
pub fn register_job_events_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "job_events",
        "get_log",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_job_event_log()
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "job_events",
        "get_by_type",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         type_ptr: i32,
         type_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let event_type = match read_wasm_string(&mut caller, type_ptr, type_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_job_events_by_type(&event_type)
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "job_events",
        "get_since",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         tick: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_job_events_since(tick as u32)
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    // entity_id is accepted for API parity but not used — the event bus is global on WasmWorld.
    linker.func_wrap(
        "job_events",
        "poll_bus",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         _entity_id: i32,
         type_ptr: i32,
         type_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let event_type = match read_wasm_string(&mut caller, type_ptr, type_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let json = {
                let mut world = caller.data().lock().unwrap();
                world.take_events(&event_type)
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "job_events",
        "clear",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> i32 {
            let mut world = caller.data().lock().unwrap();
            world.clear_job_event_log();
            0
        },
    )?;

    // deliver_callbacks: no-op in WASM (callbacks don't exist in sandbox).
    // Returns 1 always for API parity with Lua/Python.
    linker.func_wrap(
        "job_events",
        "deliver_callbacks",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> i32 {
            let world = caller.data().lock().unwrap();
            if world.deliver_callbacks() { 1 } else { 0 }
        },
    )?;

    linker.func_wrap(
        "job_events",
        "get_where",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         filter_ptr: i32,
         filter_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let filter_str = match read_wasm_string(&mut caller, filter_ptr, filter_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let filter: serde_json::Value = match serde_json::from_str(&filter_str) {
                Ok(v) => v,
                Err(_) => return -1,
            };
            let event_type = filter.get("event_type").and_then(|v| v.as_str());
            let entity_id = filter.get("entity_id").and_then(|v| v.as_u64());
            let min_tick = filter.get("min_tick").and_then(|v| v.as_u64());
            let max_tick = filter.get("max_tick").and_then(|v| v.as_u64());
            let json = {
                let world = caller.data().lock().unwrap();
                let filtered: Vec<&engine_core::ecs::world::wasm::WasmJobEvent> = world
                    .job_event_log
                    .iter()
                    .filter(|e| {
                        if let Some(et) = event_type
                            && e.event_type != et
                        {
                            return false;
                        }
                        if let Some(eid) = entity_id {
                            let matches_payload =
                                e.payload.get("entity_id").and_then(|v| v.as_u64()) == Some(eid)
                                    || e.payload.get("id").and_then(|v| v.as_u64()) == Some(eid);
                            if !matches_payload {
                                return false;
                            }
                        }
                        if let Some(min) = min_tick
                            && (e.timestamp as u64) < min
                        {
                            return false;
                        }
                        if let Some(max) = max_tick
                            && (e.timestamp as u64) > max
                        {
                            return false;
                        }
                        true
                    })
                    .collect();
                serde_json::to_string(&filtered).unwrap_or_else(|_| "[]".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    Ok(())
}
