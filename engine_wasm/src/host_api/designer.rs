use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the designer API (item definitions, equipment sets, loadouts).
pub fn register_designer_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    // load_item_definitions(dir_ptr, dir_len)
    linker.func_wrap(
        "designer",
        "load_item_definitions",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, dir_ptr: i32, dir_len: i32| {
            let dir = read_wasm_string(&mut caller, dir_ptr, dir_len)
                .expect("Failed to read dir from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .load_item_definitions_from_dir(&dir)
                .expect("Failed to load item definitions");
        },
    )?;

    // register_item(json_ptr, json_len)
    linker.func_wrap(
        "designer",
        "register_item",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, json_ptr: i32, json_len: i32| {
            let json = read_wasm_string(&mut caller, json_ptr, json_len)
                .expect("Failed to read item JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .register_item_from_json(&json)
                .expect("Failed to register item");
        },
    )?;

    // get_item_definition(id_ptr, id_len, out_ptr, out_len) -> i32
    linker.func_wrap(
        "designer",
        "get_item_definition",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         id_ptr: i32,
         id_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let id = read_wasm_string(&mut caller, id_ptr, id_len)
                .expect("Failed to read item ID from WASM memory");
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_item_definition_json(&id)
            };
            match json {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    // list_item_definitions(out_ptr, out_len) -> i32
    linker.func_wrap(
        "designer",
        "list_item_definitions",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let names = {
                let world = caller.data().lock().unwrap();
                world.list_item_names()
            };
            let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    // load_equipment_sets(dir_ptr, dir_len)
    linker.func_wrap(
        "designer",
        "load_equipment_sets",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, dir_ptr: i32, dir_len: i32| {
            let dir = read_wasm_string(&mut caller, dir_ptr, dir_len)
                .expect("Failed to read dir from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .load_equipment_sets_from_dir(&dir)
                .expect("Failed to load equipment sets");
        },
    )?;

    // register_equipment_set(json_ptr, json_len)
    linker.func_wrap(
        "designer",
        "register_equipment_set",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, json_ptr: i32, json_len: i32| {
            let json = read_wasm_string(&mut caller, json_ptr, json_len)
                .expect("Failed to read equipment set JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .register_equipment_set_from_json(&json)
                .expect("Failed to register equipment set");
        },
    )?;

    // apply_loadout(entity, name_ptr, name_len) -> i32 (entity_id)
    linker.func_wrap(
        "designer",
        "apply_loadout",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: i32,
         name_ptr: i32,
         name_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read set name from WASM memory");
            let mut world = caller.data().lock().unwrap();
            match world.apply_loadout(entity as u32, &name) {
                Ok(eid) => eid as i32,
                Err(e) => {
                    eprintln!("apply_loadout error: {e}");
                    -1
                }
            }
        },
    )?;

    // get_loadout(entity, out_ptr, out_len) -> i32
    linker.func_wrap(
        "designer",
        "get_loadout",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_loadout_json(entity as u32)
            };
            match json {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    // validate_equipment(entity, out_ptr, out_len) -> i32
    linker.func_wrap(
        "designer",
        "validate_equipment",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.validate_equipment_json(entity as u32)
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    Ok(())
}
