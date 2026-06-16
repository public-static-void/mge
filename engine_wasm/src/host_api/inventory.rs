use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the inventory API (get_inventory, set_inventory, add_item_to_inventory,
/// remove_item_from_inventory).
pub fn register_inventory_api(
    linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
) -> anyhow::Result<()> {
    linker.func_wrap(
        "inventory",
        "get_inventory",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_inventory(entity_id)
            };
            match json {
                Some(data) => {
                    let written = write_string_to_wasm(&mut caller, out_ptr, out_len, &data);
                    written as i32
                }
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "inventory",
        "set_inventory",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         json_ptr: i32,
         json_len: i32| {
            let json_data = read_wasm_string(&mut caller, json_ptr, json_len)
                .expect("Failed to read inventory JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .set_inventory(entity_id, &json_data)
                .expect("Failed to set inventory");
        },
    )?;

    linker.func_wrap(
        "inventory",
        "add_item_to_inventory",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         item_json_ptr: i32,
         item_json_len: i32| {
            let item_json = read_wasm_string(&mut caller, item_json_ptr, item_json_len)
                .expect("Failed to read item JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .add_item_to_inventory(entity_id, &item_json)
                .expect("Failed to add item to inventory");
        },
    )?;

    linker.func_wrap(
        "inventory",
        "remove_item_from_inventory",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, entity_id: u32, slot_id: i32| {
            let mut world = caller.data().lock().unwrap();
            world
                .remove_item_from_inventory(entity_id, slot_id)
                .expect("Failed to remove item from inventory");
        },
    )?;

    Ok(())
}
