use crate::host_api::component::read_wasm_string;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the world userdata API (5 host functions for map chunk and validator management).
pub fn register_world_userdata_api(
    linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
) -> anyhow::Result<()> {
    linker.func_wrap(
        "wasm_map",
        "register_map_validator",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return -1,
            };
            // Use caller.get_export() to verify the export exists and is a function
            if caller
                .get_export(&name)
                .and_then(|e| e.into_func())
                .is_none()
            {
                return -1;
            }
            let mut world = caller.data().lock().unwrap();
            let _ = world.register_map_validator(&name);
            0
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "clear_map_validators",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> i32 {
            let mut world = caller.data().lock().unwrap();
            world.clear_map_validators();
            0
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "register_map_postprocessor",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return -1,
            };
            // Use caller.get_export() to verify the export exists and is a function
            if caller
                .get_export(&name)
                .and_then(|e| e.into_func())
                .is_none()
            {
                return -1;
            }
            let mut world = caller.data().lock().unwrap();
            let _ = world.register_map_postprocessor(&name);
            0
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "clear_map_postprocessors",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> i32 {
            let mut world = caller.data().lock().unwrap();
            world.clear_map_postprocessors();
            0
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "apply_chunk",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, chunk_ptr: i32, chunk_len: i32| -> i32 {
            let chunk_json = match read_wasm_string(&mut caller, chunk_ptr, chunk_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            match world.apply_chunk(&chunk_json) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    Ok(())
}
