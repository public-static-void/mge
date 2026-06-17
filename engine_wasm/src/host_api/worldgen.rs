use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::worldgen::ThreadSafeWorldgenRegistry;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the worldgen API (2 host functions).
/// Captures the thread-safe worldgen registry in closures for list/invoke operations.
pub fn register_worldgen_api(
    linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
    worldgen_registry: Arc<Mutex<ThreadSafeWorldgenRegistry>>,
) -> anyhow::Result<()> {
    let reg = worldgen_registry.clone();
    linker.func_wrap(
        "worldgen",
        "list_worldgen_plugins",
        move |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let plugins = reg.lock().unwrap().list_names();
            let json = serde_json::to_string(&plugins).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    let reg2 = worldgen_registry.clone();
    linker.func_wrap(
        "worldgen",
        "invoke_worldgen_plugin",
        move |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
              name_ptr: i32,
              name_len: i32,
              params_ptr: i32,
              params_len: i32,
              out_ptr: i32,
              out_len: i32|
              -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read plugin name");
            let params_str = read_wasm_string(&mut caller, params_ptr, params_len)
                .expect("Failed to read params");
            let params: serde_json::Value =
                serde_json::from_str(&params_str).unwrap_or(serde_json::Value::Null);
            let result = reg2.lock().unwrap().invoke(&name, &params);
            match result {
                Ok(map) => {
                    let json = serde_json::to_string(&map).unwrap_or_default();
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
                }
                Err(_) => -1,
            }
        },
    )?;

    Ok(())
}
