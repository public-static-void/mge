use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the material API (get_properties, set_entity_material, get_entity_material, get_names).
pub fn register_material_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "material",
        "get_properties",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let json = {
                let world = caller.data().lock().unwrap();
                match world.material_definitions.get(&name) {
                    Some(props) => {
                        serde_json::to_string(props).unwrap_or_else(|_| "{}".to_string())
                    }
                    None => return -1,
                }
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "material",
        "set_entity_material",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: i32,
         name_ptr: i32,
         name_len: i32|
         -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let mut world = caller.data().lock().unwrap();
            if !world.material_definitions.contains_key(&name) {
                return -1;
            }
            let data = serde_json::json!({"material": name, "quality": 1.0});
            let json_str = data.to_string();
            match world.set_component(entity_id as u32, "Material", &json_str) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "material",
        "get_entity_material",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let data = {
                let world = caller.data().lock().unwrap();
                world.get_component(entity_id as u32, "Material")
            };
            match data {
                Some(json) => write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32,
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "material",
        "get_names",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let names: Vec<String> = {
                let world = caller.data().lock().unwrap();
                world.material_definitions.keys().cloned().collect()
            };
            let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    Ok(())
}
