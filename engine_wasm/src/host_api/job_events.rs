use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the job events API (5 functions under the `"job_events"` import module).
///
/// Functions:
/// - `get_log(out_ptr, out_len)` — returns entire job event log as JSON array
/// - `get_by_type(type_ptr, type_len, out_ptr, out_len)` — returns events filtered by type
/// - `get_since(tick, out_ptr, out_len)` — returns events with timestamp >= tick
/// - `poll_bus(entity_id, type_ptr, type_len, out_ptr, out_len)` — takes events from event bus
/// - `clear()` — clears the entire job event log
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

    Ok(())
}
