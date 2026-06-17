use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the map API (11 host functions).
pub fn register_map_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "wasm_map",
        "get_map_topology_type",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let topo = {
                let world = caller.data().lock().unwrap();
                world.get_map_topology_type()
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &topo) as i32
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "get_all_cells",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let cells = {
                let world = caller.data().lock().unwrap();
                world.get_all_cells()
            };
            let json = serde_json::to_string(&cells).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "add_cell",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, x: i32, y: i32, z: i32| {
            let mut world = caller.data().lock().unwrap();
            world.add_cell(x, y, z);
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "get_neighbors",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         cell_ptr: i32,
         cell_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let cell_json = read_wasm_string(&mut caller, cell_ptr, cell_len)
                .expect("Failed to read cell JSON from WASM memory");
            let neighbors = {
                let world = caller.data().lock().unwrap();
                world.get_neighbors(&cell_json)
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &neighbors) as i32
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "add_neighbor",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         from_ptr: i32,
         from_len: i32,
         to_ptr: i32,
         to_len: i32| {
            let from_json = read_wasm_string(&mut caller, from_ptr, from_len)
                .expect("Failed to read from cell from WASM memory");
            let to_json = read_wasm_string(&mut caller, to_ptr, to_len)
                .expect("Failed to read to cell from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .add_neighbor(&from_json, &to_json)
                .expect("Failed to add neighbor");
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "entities_in_cell",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         cell_ptr: i32,
         cell_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let cell_json = read_wasm_string(&mut caller, cell_ptr, cell_len)
                .expect("Failed to read cell JSON from WASM memory");
            let entities = {
                let world = caller.data().lock().unwrap();
                world.entities_in_cell(&cell_json)
            };
            write_u32_slice_to_wasm(&mut caller, out_ptr, &entities, out_len)
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "get_cell_metadata",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         cell_ptr: i32,
         cell_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let cell_json = read_wasm_string(&mut caller, cell_ptr, cell_len)
                .expect("Failed to read cell JSON from WASM memory");
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_cell_metadata(&cell_json)
            };
            match result {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "set_cell_metadata",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         cell_ptr: i32,
         cell_len: i32,
         meta_ptr: i32,
         meta_len: i32| {
            let cell_json = read_wasm_string(&mut caller, cell_ptr, cell_len)
                .expect("Failed to read cell JSON from WASM memory");
            let meta_json = read_wasm_string(&mut caller, meta_ptr, meta_len)
                .expect("Failed to read metadata JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world.set_cell_metadata(&cell_json, &meta_json);
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "find_path",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         start_ptr: i32,
         start_len: i32,
         goal_ptr: i32,
         goal_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let start_json = read_wasm_string(&mut caller, start_ptr, start_len)
                .expect("Failed to read start cell from WASM memory");
            let goal_json = read_wasm_string(&mut caller, goal_ptr, goal_len)
                .expect("Failed to read goal cell from WASM memory");
            let result = {
                let world = caller.data().lock().unwrap();
                world.find_path(&start_json, &goal_json)
            };
            match result {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "apply_generated_map",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, map_ptr: i32, map_len: i32| {
            let map_json = read_wasm_string(&mut caller, map_ptr, map_len)
                .expect("Failed to read map JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .apply_generated_map(&map_json)
                .expect("Failed to apply generated map");
        },
    )?;

    linker.func_wrap(
        "wasm_map",
        "get_map_cell_count",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> i32 {
            let world = caller.data().lock().unwrap();
            world.get_map_cell_count()
        },
    )?;

    Ok(())
}

fn write_u32_slice_to_wasm<T>(
    caller: &mut Caller<T>,
    ptr: i32,
    slice: &[u32],
    max_len: i32,
) -> i32 {
    let mem = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .expect("No memory export found");
    let n = std::cmp::min(slice.len(), max_len as usize);
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(slice.as_ptr() as *const u8, n * std::mem::size_of::<u32>())
    };
    mem.write(caller, ptr as usize, bytes)
        .expect("Failed to write to WASM memory");
    n as i32
}
