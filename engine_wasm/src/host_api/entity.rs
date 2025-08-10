use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the entity API
pub fn register_entity_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "entity",
        "spawn_entity",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> u32 {
            let mut world = caller.data().lock().unwrap();
            world.spawn_entity()
        },
    )?;

    linker.func_wrap(
        "entity",
        "despawn_entity",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, entity_id: u32| {
            let mut world = caller.data().lock().unwrap();
            world.despawn_entity(entity_id);
        },
    )?;

    linker.func_wrap(
        "entity",
        "get_entities",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let entities = {
                let world = caller.data().lock().unwrap();
                world.get_entities().to_vec()
            };
            write_u32_slice_to_wasm(&mut caller, out_ptr, &entities, out_len)
        },
    )?;

    linker.func_wrap(
        "entity",
        "get_entities_with_component",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read string from WASM memory");
            let entities = {
                let world = caller.data().lock().unwrap();
                world.get_entities_with_component(&name)
            };
            write_u32_slice_to_wasm(&mut caller, out_ptr, &entities, out_len)
        },
    )?;

    linker.func_wrap(
        "entity",
        "get_entities_with_components",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         names_ptr: i32,
         names_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let names_json = read_wasm_string(&mut caller, names_ptr, names_len)
                .expect("Failed to read string from WASM memory");
            let names: Vec<String> =
                serde_json::from_str(&names_json).expect("Failed to parse JSON array of names");
            let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let entities = {
                let world = caller.data().lock().unwrap();
                world.get_entities_with_components(&name_refs)
            };
            write_u32_slice_to_wasm(&mut caller, out_ptr, &entities, out_len)
        },
    )?;

    linker.func_wrap(
        "entity",
        "count_entities_with_type",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, type_ptr: i32, type_len: i32| -> i32 {
            let type_str = read_wasm_string(&mut caller, type_ptr, type_len)
                .expect("Failed to read string from WASM memory");
            let count = {
                let world = caller.data().lock().unwrap();
                world.count_entities_with_type(&type_str)
            };
            count as i32
        },
    )?;

    linker.func_wrap(
        "entity",
        "is_entity_alive",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, entity_id: u32| -> i32 {
            let alive = {
                let world = caller.data().lock().unwrap();
                world.is_entity_alive(entity_id)
            };
            alive as i32
        },
    )?;

    linker.func_wrap(
        "entity",
        "move_entity",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, entity_id: u32, dx: f32, dy: f32| {
            let mut world = caller.data().lock().unwrap();
            world.move_entity(entity_id, dx, dy);
        },
    )?;

    linker.func_wrap(
        "entity",
        "damage_entity",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, entity_id: u32, amount: f32| {
            let mut world = caller.data().lock().unwrap();
            world.damage_entity(entity_id, amount);
        },
    )?;

    Ok(())
}

fn read_wasm_string<T>(caller: &mut Caller<T>, ptr: i32, len: i32) -> anyhow::Result<String> {
    let mem = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("No memory export found"))?;
    let mut buf = vec![0u8; len as usize];
    mem.read(caller, ptr as usize, &mut buf)?;
    Ok(String::from_utf8(buf)?)
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
