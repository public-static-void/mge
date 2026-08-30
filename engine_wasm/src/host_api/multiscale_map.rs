use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::map::CellKey;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the multi-scale map navigation API (9 host functions).
///
/// Exposes the 9-function surface identically to Lua and Python (issue 61
/// parity invariant): the 4 M1 registry functions (`register_map`,
/// `set_active_map`, `get_map_names`, `get_active_map_name`) and the 5 M2
/// transition/mapping functions (`link_maps`, `enter_map`, `exit_map`,
/// `map_cell`, `unmap_cell`). Cells are CellKey JSON strings
/// (`{"Square":{"x","y","z"}}` / `{"Hex":{"q","r","z"}}` /
/// `{"Province":{"id"}}`); `map_cell` / `unmap_cell` return -1 when unlinked
/// (the WASM equivalent of nil/None); errors are deterministic status codes
/// (0 ok / -1 error), never panics.
pub fn register_multiscale_map_api(
    linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
) -> anyhow::Result<()> {
    linker.func_wrap(
        "multiscale_map",
        "register_map",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         map_ptr: i32,
         map_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read map name from WASM memory");
            let map_json = read_wasm_string(&mut caller, map_ptr, map_len)
                .expect("Failed to read map JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            match world.register_map(&name, &map_json) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "multiscale_map",
        "set_active_map",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read map name from WASM memory");
            let mut world = caller.data().lock().unwrap();
            match world.set_active_map(&name) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "multiscale_map",
        "get_map_names",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let names = {
                let world = caller.data().lock().unwrap();
                world.get_map_names()
            };
            let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "multiscale_map",
        "get_active_map_name",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let name = {
                let world = caller.data().lock().unwrap();
                world.get_active_map_name()
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &name) as i32
        },
    )?;

    linker.func_wrap(
        "multiscale_map",
        "link_maps",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         source_map_ptr: i32,
         source_map_len: i32,
         source_cell_ptr: i32,
         source_cell_len: i32,
         target_map_ptr: i32,
         target_map_len: i32,
         target_cell_ptr: i32,
         target_cell_len: i32|
         -> i32 {
            let source_map = read_wasm_string(&mut caller, source_map_ptr, source_map_len)
                .expect("Failed to read source map name from WASM memory");
            let source_cell_json = read_wasm_string(&mut caller, source_cell_ptr, source_cell_len)
                .expect("Failed to read source cell from WASM memory");
            let target_map = read_wasm_string(&mut caller, target_map_ptr, target_map_len)
                .expect("Failed to read target map name from WASM memory");
            let target_cell_json = read_wasm_string(&mut caller, target_cell_ptr, target_cell_len)
                .expect("Failed to read target cell from WASM memory");
            let source_cell: CellKey = match serde_json::from_str(&source_cell_json) {
                Ok(c) => c,
                Err(_) => return -1,
            };
            let target_cell: CellKey = match serde_json::from_str(&target_cell_json) {
                Ok(c) => c,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            match world.link_maps(&source_map, source_cell, &target_map, target_cell) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "multiscale_map",
        "enter_map",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         entry_cell_ptr: i32,
         entry_cell_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read map name from WASM memory");
            let entry_cell_json = read_wasm_string(&mut caller, entry_cell_ptr, entry_cell_len)
                .expect("Failed to read entry cell from WASM memory");
            let entry_cell: CellKey = match serde_json::from_str(&entry_cell_json) {
                Ok(c) => c,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            match world.enter_map(&name, entry_cell) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "multiscale_map",
        "exit_map",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> i32 {
            let mut world = caller.data().lock().unwrap();
            match world.exit_map() {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "multiscale_map",
        "map_cell",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         source_map_ptr: i32,
         source_map_len: i32,
         source_cell_ptr: i32,
         source_cell_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let source_map = read_wasm_string(&mut caller, source_map_ptr, source_map_len)
                .expect("Failed to read source map name from WASM memory");
            let source_cell_json = read_wasm_string(&mut caller, source_cell_ptr, source_cell_len)
                .expect("Failed to read source cell from WASM memory");
            let source_cell: CellKey = match serde_json::from_str(&source_cell_json) {
                Ok(c) => c,
                Err(_) => return -1,
            };
            let result = {
                let world = caller.data().lock().unwrap();
                world.map_cell(&source_map, &source_cell)
            };
            match result {
                Some(cell) => {
                    let json = serde_json::to_string(&cell).unwrap_or_default();
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
                }
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "multiscale_map",
        "unmap_cell",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         target_map_ptr: i32,
         target_map_len: i32,
         target_cell_ptr: i32,
         target_cell_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let target_map = read_wasm_string(&mut caller, target_map_ptr, target_map_len)
                .expect("Failed to read target map name from WASM memory");
            let target_cell_json = read_wasm_string(&mut caller, target_cell_ptr, target_cell_len)
                .expect("Failed to read target cell from WASM memory");
            let target_cell: CellKey = match serde_json::from_str(&target_cell_json) {
                Ok(c) => c,
                Err(_) => return -1,
            };
            let result = {
                let world = caller.data().lock().unwrap();
                world.unmap_cell(&target_map, &target_cell)
            };
            match result {
                Some(cell) => {
                    let json = serde_json::to_string(&cell).unwrap_or_default();
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
                }
                None => -1,
            }
        },
    )?;

    Ok(())
}
