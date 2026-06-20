use crate::host_api::component::write_string_to_wasm;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the UI tree API (parent-child widget hierarchy) on the "ui_tree" import module.
pub fn register_ui_tree_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    // 1. add_child(parent_id: i32, child_id: i32) -> i32 (1=ok, 0=fail)
    linker.func_wrap(
        "ui_tree",
        "add_child",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, parent_id: i32, child_id: i32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            if world.ui_add_child(parent_id as u32, child_id as u32) {
                1
            } else {
                0
            }
        },
    )?;

    // 2. get_children(widget_id: i32, out_ptr: i32, out_len: i32) -> i32
    //    Returns bytes written, or -1 if widget not found.
    linker.func_wrap(
        "ui_tree",
        "get_children",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         widget_id: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let children = {
                let world = caller.data().lock().unwrap();
                world.ui_get_children(widget_id as u32)
            };
            match children {
                Some(json) => write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32,
                None => -1,
            }
        },
    )?;

    // 3. remove_child(parent_id: i32, child_id: i32) -> i32 (1=ok, 0=fail)
    linker.func_wrap(
        "ui_tree",
        "remove_child",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, parent_id: i32, child_id: i32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            if world.ui_remove_child(parent_id as u32, child_id as u32) {
                1
            } else {
                0
            }
        },
    )?;

    // 4. get_parent(widget_id: i32) -> i32
    //    Returns parent_id (positive) or -1 if no parent / not found.
    linker.func_wrap(
        "ui_tree",
        "get_parent",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, widget_id: i32| -> i32 {
            let world = caller.data().lock().unwrap();
            match world.ui_get_parent(widget_id as u32) {
                Some(parent_id) => parent_id as i32,
                None => -1,
            }
        },
    )?;

    Ok(())
}
