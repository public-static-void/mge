use crate::host_api::component::read_wasm_string;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the movement operations API
/// (assign_move_path, is_agent_at_cell, is_move_path_empty).
pub fn register_movement_ops_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "movement_ops",
        "assign_move_path",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         agent_id: u32,
         path_ptr: i32,
         path_len: i32| {
            let path_json = read_wasm_string(&mut caller, path_ptr, path_len)
                .expect("Failed to read move path JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .assign_move_path(agent_id, &path_json)
                .expect("Failed to assign move path");
        },
    )?;

    linker.func_wrap(
        "movement_ops",
        "is_agent_at_cell",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         agent_id: u32,
         cell_ptr: i32,
         cell_len: i32|
         -> i32 {
            let cell_json = read_wasm_string(&mut caller, cell_ptr, cell_len)
                .expect("Failed to read cell JSON from WASM memory");
            let world = caller.data().lock().unwrap();
            if world.is_agent_at_cell(agent_id, &cell_json) {
                1
            } else {
                0
            }
        },
    )?;

    linker.func_wrap(
        "movement_ops",
        "is_move_path_empty",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, agent_id: u32| -> i32 {
            let world = caller.data().lock().unwrap();
            if world.is_move_path_empty(agent_id) {
                1
            } else {
                0
            }
        },
    )?;

    Ok(())
}
