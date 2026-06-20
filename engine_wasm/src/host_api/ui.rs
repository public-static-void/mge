use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the UI API (widget management) on the "ui" import module.
pub fn register_ui_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    // 1. create_widget(type_ptr, type_len, props_ptr, props_len) -> i32
    //    Returns widget_id (positive) or 0 on error.
    linker.func_wrap(
        "ui",
        "create_widget",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         type_ptr: i32,
         type_len: i32,
         props_ptr: i32,
         props_len: i32|
         -> i32 {
            let widget_type = match read_wasm_string(&mut caller, type_ptr, type_len) {
                Ok(s) => s,
                Err(_) => return 0,
            };
            let props = match read_wasm_string(&mut caller, props_ptr, props_len) {
                Ok(s) => s,
                Err(_) => return 0,
            };
            let mut world = caller.data().lock().unwrap();
            world.ui_create_widget(&widget_type, &props) as i32
        },
    )?;

    // 2. remove_widget(widget_id: i32) -> i32 (1=ok, 0=not_found)
    linker.func_wrap(
        "ui",
        "remove_widget",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, widget_id: i32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            if world.ui_remove_widget(widget_id as u32) {
                1
            } else {
                0
            }
        },
    )?;

    // 3. set_widget_props(widget_id: i32, props_ptr: i32, props_len: i32) -> i32 (1=ok, 0=not_found)
    linker.func_wrap(
        "ui",
        "set_widget_props",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         widget_id: i32,
         props_ptr: i32,
         props_len: i32|
         -> i32 {
            let props = match read_wasm_string(&mut caller, props_ptr, props_len) {
                Ok(s) => s,
                Err(_) => return 0,
            };
            let mut world = caller.data().lock().unwrap();
            if world.ui_set_widget_props(widget_id as u32, &props) {
                1
            } else {
                0
            }
        },
    )?;

    // 4. get_widget_props(widget_id: i32, out_ptr: i32, out_len: i32) -> i32
    //    Returns bytes written, or -1 if widget not found.
    linker.func_wrap(
        "ui",
        "get_widget_props",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         widget_id: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let props = {
                let world = caller.data().lock().unwrap();
                world.ui_get_widget_props(widget_id as u32)
            };
            match props {
                Some(json_str) => {
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
                }
                None => -1,
            }
        },
    )?;

    // 5. get_widget_type(widget_id: i32, out_ptr: i32, out_len: i32) -> i32
    //    Returns bytes written, or -1 if widget not found.
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
                Some(name) => write_string_to_wasm(&mut caller, out_ptr, out_len, &name) as i32,
                None => -1,
            }
        },
    )?;

    // 6. load_json(json_ptr: i32, json_len: i32, out_ptr: i32, out_len: i32) -> i32
    //    Returns bytes written, or -1 on parse failure.
    linker.func_wrap(
        "ui",
        "load_json",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         json_ptr: i32,
         json_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json_str = match read_wasm_string(&mut caller, json_ptr, json_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let result = {
                let mut world = caller.data().lock().unwrap();
                world.ui_load_json(&json_str)
            };
            match result {
                Some(ids_json) => {
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &ids_json) as i32
                }
                None => -1,
            }
        },
    )?;

    // 7. set_z_order(widget_id: i32, z: i32) -> i32 (1=ok, 0=not_found)
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

    // 8. get_z_order(widget_id: i32) -> i32
    //    Returns z_order value, or 0 if widget not found.
    linker.func_wrap(
        "ui",
        "get_z_order",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, widget_id: i32| -> i32 {
            let world = caller.data().lock().unwrap();
            world.ui_get_z_order(widget_id as u32)
        },
    )?;

    // 9. register_widget(type_ptr: i32, type_len: i32) -> i32 (1=registered, 0=already_exists)
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
