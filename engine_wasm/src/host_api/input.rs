use crate::host_api::component::write_string_to_wasm;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the input API (get_user_input).
pub fn register_input_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "input",
        "get_user_input",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let line = {
                let mut world = caller.data().lock().unwrap();
                world.get_user_input()
            };
            match line {
                Some(data) => {
                    let written = write_string_to_wasm(&mut caller, out_ptr, out_len, &data);
                    written as i32
                }
                None => -1,
            }
        },
    )?;

    Ok(())
}
