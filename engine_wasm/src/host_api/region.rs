use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the region API (4 host functions).
///
/// Zone management adds 8 more host functions under the same `region`
/// module with identical names to the Lua/Python surface:
/// `designate_zone`, `remove_zone`, `rename_zone`, `set_zone_kind`,
/// `assign_cells_to_zone`, `unassign_cells_from_zone`, `list_zones`,
/// `get_zone`. `shape`/`cells` arguments and `list_zones`/`get_zone`
/// returns travel as JSON strings: `shape` is
/// `{"rect": {x0, y0, z, x1, y1}}` or `{"cells": [...]}`; `label` for
/// `designate_zone` is JSON (`"name"` or `null`). Boolean results are
/// tri-state: `1` true, `0` false (unknown id), `-1` gate/validation
/// error. `get_zone` writes the zone JSON on success and returns `-1`
/// for unknown ids or gate errors.
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

    linker.func_wrap(
        "region",
        "designate_zone",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         kind_ptr: i32,
         kind_len: i32,
         label_ptr: i32,
         label_len: i32,
         shape_ptr: i32,
         shape_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let kind =
                read_wasm_string(&mut caller, kind_ptr, kind_len).expect("Failed to read kind");
            let label_json =
                read_wasm_string(&mut caller, label_ptr, label_len).expect("Failed to read label");
            let shape_json =
                read_wasm_string(&mut caller, shape_ptr, shape_len).expect("Failed to read shape");
            let label: Option<String> = match serde_json::from_str(&label_json) {
                Ok(label) => label,
                Err(_) => return -1,
            };
            let zone_id = {
                let mut world = caller.data().lock().unwrap();
                match world.designate_zone(&kind, label.as_deref(), &shape_json) {
                    Ok(id) => id,
                    Err(_) => return -1,
                }
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &zone_id) as i32
        },
    )?;

    linker.func_wrap(
        "region",
        "remove_zone",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         zone_id_ptr: i32,
         zone_id_len: i32|
         -> i32 {
            let zone_id = read_wasm_string(&mut caller, zone_id_ptr, zone_id_len)
                .expect("Failed to read zone_id");
            let mut world = caller.data().lock().unwrap();
            match world.remove_zone(&zone_id) {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "region",
        "rename_zone",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         zone_id_ptr: i32,
         zone_id_len: i32,
         label_ptr: i32,
         label_len: i32|
         -> i32 {
            let zone_id = read_wasm_string(&mut caller, zone_id_ptr, zone_id_len)
                .expect("Failed to read zone_id");
            let label =
                read_wasm_string(&mut caller, label_ptr, label_len).expect("Failed to read label");
            let mut world = caller.data().lock().unwrap();
            match world.rename_zone(&zone_id, &label) {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "region",
        "set_zone_kind",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         zone_id_ptr: i32,
         zone_id_len: i32,
         kind_ptr: i32,
         kind_len: i32|
         -> i32 {
            let zone_id = read_wasm_string(&mut caller, zone_id_ptr, zone_id_len)
                .expect("Failed to read zone_id");
            let kind =
                read_wasm_string(&mut caller, kind_ptr, kind_len).expect("Failed to read kind");
            let mut world = caller.data().lock().unwrap();
            match world.set_zone_kind(&zone_id, &kind) {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "region",
        "assign_cells_to_zone",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         zone_id_ptr: i32,
         zone_id_len: i32,
         cells_ptr: i32,
         cells_len: i32|
         -> i32 {
            let zone_id = read_wasm_string(&mut caller, zone_id_ptr, zone_id_len)
                .expect("Failed to read zone_id");
            let cells_json =
                read_wasm_string(&mut caller, cells_ptr, cells_len).expect("Failed to read cells");
            let mut world = caller.data().lock().unwrap();
            match world.assign_cells_to_zone(&zone_id, &cells_json) {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "region",
        "unassign_cells_from_zone",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         zone_id_ptr: i32,
         zone_id_len: i32,
         cells_ptr: i32,
         cells_len: i32|
         -> i32 {
            let zone_id = read_wasm_string(&mut caller, zone_id_ptr, zone_id_len)
                .expect("Failed to read zone_id");
            let cells_json =
                read_wasm_string(&mut caller, cells_ptr, cells_len).expect("Failed to read cells");
            let mut world = caller.data().lock().unwrap();
            match world.unassign_cells_from_zone(&zone_id, &cells_json) {
                Ok(true) => 1,
                Ok(false) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "region",
        "list_zones",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let zones = {
                let world = caller.data().lock().unwrap();
                world.list_zones()
            };
            let json = serde_json::to_string(&zones).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "region",
        "get_zone",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         zone_id_ptr: i32,
         zone_id_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let zone_id = read_wasm_string(&mut caller, zone_id_ptr, zone_id_len)
                .expect("Failed to read zone_id");
            let zone = {
                let world = caller.data().lock().unwrap();
                world.get_zone(&zone_id)
            };
            match zone {
                Some(zone) => {
                    let json = serde_json::to_string(&zone).unwrap_or_else(|_| "{}".to_string());
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
                }
                None => -1,
            }
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
