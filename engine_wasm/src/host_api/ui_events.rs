use crate::host_api::component::read_wasm_string;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the UI events API (interaction and callbacks) on the "ui_events" import module.
pub fn register_ui_events_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    // 1. set_callback(widget_id: i32, event_type_ptr: i32, event_type_len: i32) -> i32
    //    No-op stub: callbacks are not used in WASM (AD007 poll-based design).
    linker.func_wrap(
        "ui_events",
        "set_callback",
        |_caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         _widget_id: i32,
         _event_type_ptr: i32,
         _event_type_len: i32|
         -> i32 { 0 },
    )?;

    // 2. remove_callback(widget_id: i32, event_type_ptr: i32, event_type_len: i32) -> i32
    //    No-op stub: no callbacks to remove.
    linker.func_wrap(
        "ui_events",
        "remove_callback",
        |_caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         _widget_id: i32,
         _event_type_ptr: i32,
         _event_type_len: i32|
         -> i32 { 0 },
    )?;

    // 3. focus_widget(widget_id: i32) -> i32 (1=focused, 0=not_found)
    linker.func_wrap(
        "ui_events",
        "focus_widget",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, widget_id: i32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            if world.ui_focus_widget(widget_id as u32) {
                1
            } else {
                0
            }
        },
    )?;

    // 4. trigger_event(widget_id: i32, event_type_ptr: i32, event_type_len: i32,
    //                   event_data_ptr: i32, event_data_len: i32) -> i32
    //    Returns 1 if event queued, 0 if widget not found.
    linker.func_wrap(
        "ui_events",
        "trigger_event",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         widget_id: i32,
         event_type_ptr: i32,
         event_type_len: i32,
         event_data_ptr: i32,
         event_data_len: i32|
         -> i32 {
            let event_type = match read_wasm_string(&mut caller, event_type_ptr, event_type_len) {
                Ok(s) => s,
                Err(_) => return 0,
            };
            let event_data = match read_wasm_string(&mut caller, event_data_ptr, event_data_len) {
                Ok(s) => s,
                Err(_) => return 0,
            };
            let mut world = caller.data().lock().unwrap();
            if world.ui_trigger_event(widget_id as u32, &event_type, &event_data) {
                1
            } else {
                0
            }
        },
    )?;

    Ok(())
}
