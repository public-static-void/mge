use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the event bus API (send_event, poll_event, update_event_buses).
pub fn register_event_bus_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "event_bus",
        "send_event",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         type_ptr: i32,
         type_len: i32,
         data_ptr: i32,
         data_len: i32| {
            let event_type = read_wasm_string(&mut caller, type_ptr, type_len)
                .expect("Failed to read event type from WASM memory");
            let event_data = read_wasm_string(&mut caller, data_ptr, data_len)
                .expect("Failed to read event data from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .send_event(&event_type, &event_data)
                .expect("Failed to send event");
        },
    )?;

    linker.func_wrap(
        "event_bus",
        "poll_event",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         type_ptr: i32,
         type_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let event_type = read_wasm_string(&mut caller, type_ptr, type_len)
                .expect("Failed to read event type from WASM memory");
            let json = {
                let world = caller.data().lock().unwrap();
                world.poll_event(&event_type)
            };
            let written = write_string_to_wasm(&mut caller, out_ptr, out_len, &json);
            written as i32
        },
    )?;

    linker.func_wrap(
        "event_bus",
        "update_event_buses",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| {
            let mut world = caller.data().lock().unwrap();
            world.update_event_buses();
        },
    )?;

    Ok(())
}
