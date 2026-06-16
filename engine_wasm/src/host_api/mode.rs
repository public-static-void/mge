use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the mode API (set_mode, get_mode, get_available_modes).
pub fn register_mode_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "mode",
        "set_mode",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, mode_ptr: i32, mode_len: i32| {
            let mode = read_wasm_string(&mut caller, mode_ptr, mode_len)
                .expect("Failed to read mode string from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world.set_mode(&mode);
        },
    )?;

    linker.func_wrap(
        "mode",
        "get_mode",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let mode = {
                let world = caller.data().lock().unwrap();
                world.get_mode().to_string()
            };
            let written = write_string_to_wasm(&mut caller, out_ptr, out_len, &mode);
            written as i32
        },
    )?;

    linker.func_wrap(
        "mode",
        "get_available_modes",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let modes = {
                let world = caller.data().lock().unwrap();
                world.get_available_modes()
            };
            let json = serde_json::to_string(&modes).unwrap_or_else(|_| "[]".to_string());
            let written = write_string_to_wasm(&mut caller, out_ptr, out_len, &json);
            written as i32
        },
    )?;

    Ok(())
}
