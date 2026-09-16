use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the construction API (4 host functions under `construction`).
///
/// Transport is JSON strings, matching the surrounding host APIs:
/// `place_blueprint` takes JSON-encoded cell and materials and returns the
/// site id (`-1` on validation or mode-gated error);
/// `get_construction_state` writes `{ state, progress, required_work,
/// building_type }` JSON into the out buffer (`-1` on unknown id or
/// mode-gated error); `cancel_construction` / `demolish_building` return
/// `0` on success and `-1` on error.
pub fn register_construction_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "construction",
        "place_blueprint",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         building_type_ptr: i32,
         building_type_len: i32,
         cell_ptr: i32,
         cell_len: i32,
         materials_ptr: i32,
         materials_len: i32,
         required_work: i64|
         -> i32 {
            let building_type = read_wasm_string(&mut caller, building_type_ptr, building_type_len)
                .expect("Failed to read building type from WASM memory");
            let cell_json = read_wasm_string(&mut caller, cell_ptr, cell_len)
                .expect("Failed to read cell JSON from WASM memory");
            let materials_json = read_wasm_string(&mut caller, materials_ptr, materials_len)
                .expect("Failed to read materials JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            match world.place_blueprint(&building_type, &cell_json, &materials_json, required_work)
            {
                Ok(site_id) => site_id as i32,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "construction",
        "get_construction_state",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         site_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_construction_state(site_id)
            };
            match result {
                Ok(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "construction",
        "cancel_construction",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, site_id: u32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            match world.cancel_construction(site_id) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "construction",
        "demolish_building",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, building_id: u32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            match world.demolish_building(building_id) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        },
    )?;

    Ok(())
}
