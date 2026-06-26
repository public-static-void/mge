use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::loot::LootEntry;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the loot table API (define_table, roll, has_table, table_names, remove_table).
pub fn register_loot_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "loot",
        "define_table",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         entries_ptr: i32,
         entries_len: i32| {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read table name from WASM memory");
            let entries_json = read_wasm_string(&mut caller, entries_ptr, entries_len)
                .expect("Failed to read entries JSON from WASM memory");
            let entries: Vec<LootEntry> =
                serde_json::from_str(&entries_json).expect("Failed to parse loot entries JSON");
            let mut world = caller.data().lock().unwrap();
            world
                .loot_tables
                .define_table(&name, entries)
                .expect("Failed to define loot table");
        },
    )?;

    linker.func_wrap(
        "loot",
        "roll",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read table name from WASM memory");
            let result = {
                let world = caller.data().lock().unwrap();
                world.loot_tables.roll(&name)
            };
            match result {
                Ok(items) => {
                    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
                }
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "loot",
        "has_table",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read table name from WASM memory");
            let world = caller.data().lock().unwrap();
            if world.loot_tables.has_table(&name) {
                1
            } else {
                0
            }
        },
    )?;

    linker.func_wrap(
        "loot",
        "table_names",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let names = {
                let world = caller.data().lock().unwrap();
                world.loot_tables.table_names()
            };
            let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "loot",
        "remove_table",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read table name from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world.loot_tables.remove_table(&name);
        },
    )?;

    Ok(())
}
