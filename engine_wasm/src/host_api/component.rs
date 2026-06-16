use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the component API
pub fn register_component_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "component",
        "set_component",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         name_ptr: i32,
         name_len: i32,
         json_ptr: i32,
         json_len: i32| {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read component name from WASM memory");
            let json_data = read_wasm_string(&mut caller, json_ptr, json_len)
                .expect("Failed to read component JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .set_component(entity_id, &name, &json_data)
                .expect("Failed to set component");
        },
    )?;

    linker.func_wrap(
        "component",
        "get_component",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         name_ptr: i32,
         name_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read component name from WASM memory");
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_component(entity_id, &name)
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
        "component",
        "remove_component",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         name_ptr: i32,
         name_len: i32| {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read component name from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .remove_component(entity_id, &name)
                .expect("Failed to remove component");
        },
    )?;

    Ok(())
}

pub fn read_wasm_string<T>(caller: &mut Caller<T>, ptr: i32, len: i32) -> anyhow::Result<String> {
    let mem = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("No memory export found"))?;
    let mut buf = vec![0u8; len as usize];
    mem.read(caller, ptr as usize, &mut buf)?;
    Ok(String::from_utf8(buf)?)
}

pub fn write_string_to_wasm<T>(
    caller: &mut Caller<T>,
    ptr: i32,
    max_len: i32,
    data: &str,
) -> usize {
    let mem = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .expect("No memory export found");
    let bytes = data.as_bytes();
    let n = std::cmp::min(bytes.len(), max_len as usize);
    mem.write(caller, ptr as usize, &bytes[..n])
        .expect("Failed to write to WASM memory");
    n
}
