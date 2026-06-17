use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the equipment API
/// (get_equipment, set_equipment, equip_item, unequip_item).
pub fn register_equipment_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "equipment",
        "get_equipment",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_equipment(entity_id)
            };
            match json {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "equipment",
        "set_equipment",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         json_ptr: i32,
         json_len: i32| {
            let json_data = read_wasm_string(&mut caller, json_ptr, json_len)
                .expect("Failed to read equipment JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .set_equipment(entity_id, &json_data)
                .expect("Failed to set equipment");
        },
    )?;

    linker.func_wrap(
        "equipment",
        "equip_item",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         item_id_ptr: i32,
         item_id_len: i32,
         slot_ptr: i32,
         slot_len: i32| {
            let item_id = read_wasm_string(&mut caller, item_id_ptr, item_id_len)
                .expect("Failed to read item_id from WASM memory");
            let slot = read_wasm_string(&mut caller, slot_ptr, slot_len)
                .expect("Failed to read slot from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .equip_item(entity_id, &item_id, &slot)
                .expect("Failed to equip item");
        },
    )?;

    linker.func_wrap(
        "equipment",
        "unequip_item",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         slot_ptr: i32,
         slot_len: i32| {
            let slot = read_wasm_string(&mut caller, slot_ptr, slot_len)
                .expect("Failed to read slot from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .unequip_item(entity_id, &slot)
                .expect("Failed to unequip item");
        },
    )?;

    Ok(())
}
