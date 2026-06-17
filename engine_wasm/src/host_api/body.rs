use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the body API (5 host functions).
pub fn register_body_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "body",
        "get_body",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_body(entity_id)
            };
            match result {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "body",
        "set_body",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         json_ptr: i32,
         json_len: i32| {
            let json_data = read_wasm_string(&mut caller, json_ptr, json_len)
                .expect("Failed to read body JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .set_body(entity_id, &json_data)
                .expect("Failed to set body");
        },
    )?;

    linker.func_wrap(
        "body",
        "add_body_part",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         part_ptr: i32,
         part_len: i32| {
            let part_json = read_wasm_string(&mut caller, part_ptr, part_len)
                .expect("Failed to read part JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .add_body_part(entity_id, &part_json)
                .expect("Failed to add body part");
        },
    )?;

    linker.func_wrap(
        "body",
        "remove_body_part",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         name_ptr: i32,
         name_len: i32| {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read part name from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .remove_body_part(entity_id, &name)
                .expect("Failed to remove body part");
        },
    )?;

    linker.func_wrap(
        "body",
        "get_body_part",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         name_ptr: i32,
         name_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read part name from WASM memory");
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_body_part(entity_id, &name)
            };
            match result {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    Ok(())
}
