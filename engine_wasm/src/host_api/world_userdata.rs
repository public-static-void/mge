use crate::host_api::component::read_wasm_string;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the world userdata API (5 host functions for map chunk and validator management).
pub fn register_world_userdata_api(
    linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
) -> anyhow::Result<()> {
    linker.func_wrap(
        "wasm_world_userdata",
        "register_map_validator",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            if !world.discovered_export_names.contains(&name) {
                return -1;
            }
            world.register_map_validator(&name);
            0
        },
    )?;

    linker.func_wrap(
        "wasm_world_userdata",
        "clear_map_validators",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| {
            let mut world = caller.data().lock().unwrap();
            world.clear_map_validators();
        },
    )?;

    linker.func_wrap(
        "wasm_world_userdata",
        "register_map_postprocessor",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            if !world.discovered_export_names.contains(&name) {
                return -1;
            }
            world.register_map_postprocessor(&name);
            0
        },
    )?;

    linker.func_wrap(
        "wasm_world_userdata",
        "clear_map_postprocessors",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| {
            let mut world = caller.data().lock().unwrap();
            world.clear_map_postprocessors();
        },
    )?;

    linker.func_wrap(
        "wasm_world_userdata",
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
