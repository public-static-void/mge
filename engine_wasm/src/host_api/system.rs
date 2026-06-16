use crate::host_api::component::read_wasm_string;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the system API (register_system, run_system).
pub fn register_system_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "system",
        "register_system",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         type_ptr: i32,
         type_len: i32| {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read system name from WASM memory");
            let system_type = read_wasm_string(&mut caller, type_ptr, type_len)
                .expect("Failed to read system type from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world.register_system(&name, &system_type);
        },
    )?;

    linker.func_wrap(
        "system",
        "run_system",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read system name from WASM memory");
            let world = caller.data().lock().unwrap();
            world.run_system(&name).expect("Failed to run system");
        },
    )?;

    Ok(())
}
