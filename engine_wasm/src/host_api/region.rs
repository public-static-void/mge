use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the region API (4 host functions).
pub fn register_region_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "region",
        "get_entities_in_region",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         region_id_ptr: i32,
         region_id_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let region_id = read_wasm_string(&mut caller, region_id_ptr, region_id_len)
                .expect("Failed to read region_id");
            let entities = {
                let world = caller.data().lock().unwrap();
                world.entities_in_region(&region_id)
            };
            write_u32_slice_to_wasm(&mut caller, out_ptr, &entities, out_len)
        },
    )?;

    linker.func_wrap(
        "region",
        "get_entities_in_region_kind",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         kind_ptr: i32,
         kind_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let kind =
                read_wasm_string(&mut caller, kind_ptr, kind_len).expect("Failed to read kind");
            let entities = {
                let world = caller.data().lock().unwrap();
                world.entities_in_region_kind(&kind)
            };
            write_u32_slice_to_wasm(&mut caller, out_ptr, &entities, out_len)
        },
    )?;

    linker.func_wrap(
        "region",
        "get_cells_in_region",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         region_id_ptr: i32,
         region_id_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let region_id = read_wasm_string(&mut caller, region_id_ptr, region_id_len)
                .expect("Failed to read region_id");
            let cells = {
                let world = caller.data().lock().unwrap();
                world.cells_in_region(&region_id)
            };
            let json = serde_json::to_string(&cells).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "region",
        "get_cells_in_region_kind",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         kind_ptr: i32,
         kind_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let kind =
                read_wasm_string(&mut caller, kind_ptr, kind_len).expect("Failed to read kind");
            let cells = {
                let world = caller.data().lock().unwrap();
                world.cells_in_region_kind(&kind)
            };
            let json = serde_json::to_string(&cells).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
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
