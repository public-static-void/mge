use crate::host_api::component::read_wasm_string;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the save/load API (save_to_file, load_from_file).
pub fn register_save_load_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "save_load",
        "save_to_file",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, path_ptr: i32, path_len: i32| {
            let path = read_wasm_string(&mut caller, path_ptr, path_len)
                .expect("Failed to read path from WASM memory");
            let world = caller.data().lock().unwrap();
            world
                .save_to_file(&path)
                .expect("Failed to save world to file");
        },
    )?;

    linker.func_wrap(
        "save_load",
        "load_from_file",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, path_ptr: i32, path_len: i32| {
            let path = read_wasm_string(&mut caller, path_ptr, path_len)
                .expect("Failed to read path from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .load_from_file(&path)
                .expect("Failed to load world from file");
        },
    )?;

    Ok(())
}
