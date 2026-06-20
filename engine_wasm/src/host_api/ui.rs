use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the UI widget API (get_widget_type, set_z_order, get_z_order, register_widget).
pub fn register_ui_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "ui",
        "get_widget_type",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         widget_id: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let type_name = {
                let world = caller.data().lock().unwrap();
                world.ui_get_widget_type(widget_id as u32)
            };
            match type_name {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "ui",
        "set_z_order",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, widget_id: i32, z: i32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            if world.ui_set_z_order(widget_id as u32, z) {
                1
            } else {
                0
            }
        },
    )?;

    linker.func_wrap(
        "ui",
        "get_z_order",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, widget_id: i32| -> i32 {
            let world = caller.data().lock().unwrap();
            world.ui_get_z_order(widget_id as u32)
        },
    )?;

    linker.func_wrap(
        "ui",
        "register_widget",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         type_ptr: i32,
         type_len: i32|
         -> i32 {
            let type_name = match read_wasm_string(&mut caller, type_ptr, type_len) {
                Ok(s) => s,
                Err(_) => return 0,
            };
            let mut world = caller.data().lock().unwrap();
            if world.ui_register_widget_type(&type_name) {
                1
            } else {
                0
            }
        },
    )?;

    Ok(())
}
