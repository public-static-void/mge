use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the UI tree API (get_parent).
pub fn register_ui_tree_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
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
