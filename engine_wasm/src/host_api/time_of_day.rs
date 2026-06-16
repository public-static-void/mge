use crate::host_api::component::write_string_to_wasm;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the time of day API (get_time_of_day).
pub fn register_time_of_day_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "time_of_day",
        "get_time_of_day",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let time = {
                let world = caller.data().lock().unwrap();
                world.get_time_of_day()
            };
            let json = serde_json::to_string(&time).unwrap_or_else(|_| "{}".to_string());
            let written = write_string_to_wasm(&mut caller, out_ptr, out_len, &json);
            written as i32
        },
    )?;

    Ok(())
}
