use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::worldgen::ThreadSafeWorldgenRegistry;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker, Val};

/// Known worldgen export names.
const EXPORT_WORLDGEN_GENERATE: &str = "mge_worldgen_generate";
const EXPORT_WORLDGEN_VALIDATE: &str = "mge_worldgen_validate";
const EXPORT_WORLDGEN_POSTPROCESS: &str = "mge_worldgen_postprocess";

/// Scratch buffer offset in WASM linear memory for passing data to guest exports.
const SCRATCH_INPUT_OFFSET: usize = 1024;
/// Scratch buffer offset for output from guest exports.
const SCRATCH_OUTPUT_OFFSET: usize = 4096;
/// Maximum output buffer size.
const SCRATCH_OUTPUT_MAX: i32 = 4096;

/// Registers the worldgen API (5 host functions: list, invoke, and 3 registration functions).
pub fn register_worldgen_api(
    linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
    worldgen_registry: Arc<Mutex<ThreadSafeWorldgenRegistry>>,
) -> anyhow::Result<()> {
    let reg = worldgen_registry.clone();
    linker.func_wrap(
        "worldgen",
        "list_worldgen_plugins",
        move |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            // Combine registry plugins with WASM worldgen plugins
            let mut plugins = reg.lock().unwrap().list_names();
            let world = caller.data().lock().unwrap();
            plugins.extend(world.wasm_worldgen_plugins.clone());
            drop(world);
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

            // Try registered plugins first (CAbi, Lua, Scripting)
            let result = reg2.lock().unwrap().invoke(&name, &params);
            match result {
                Ok(map) => {
                    let json = serde_json::to_string(&map).unwrap_or_default();
                    return write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32;
                }
                Err(engine_core::worldgen::WorldgenError::NotFound) => {
                    // Fall through to check WASM plugins
                }
                Err(_) => return -1,
            }

            // Check for WASM-based plugin
            let is_wasm = {
                let world = caller.data().lock().unwrap();
                world.wasm_worldgen_plugins.contains(&name)
            };
            if !is_wasm {
                return -1;
            }

            // Call the WASM mge_worldgen_generate export
            let generate_func = match caller
                .get_export(EXPORT_WORLDGEN_GENERATE)
                .and_then(|e| e.into_func())
            {
                Some(f) => f,
                None => return -1,
            };
            let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                Some(m) => m,
                None => return -1,
            };

            // Write params JSON to scratch buffer
            let params_json = serde_json::to_string(&params).unwrap_or_default();
            let params_bytes = params_json.as_bytes();
            if memory
                .write(&mut caller, SCRATCH_INPUT_OFFSET, params_bytes)
                .is_err()
            {
                return -1;
            }

            // Call generate export
            let mut results = [Val::I32(0)];
            if generate_func
                .call(
                    &mut caller,
                    &[
                        Val::I32(SCRATCH_INPUT_OFFSET as i32),
                        Val::I32(params_bytes.len() as i32),
                        Val::I32(SCRATCH_OUTPUT_OFFSET as i32),
                        Val::I32(SCRATCH_OUTPUT_MAX),
                    ],
                    &mut results,
                )
                .is_err()
            {
                return -1;
            }

            let bytes_written = match results[0] {
                Val::I32(n) if n >= 0 => n as usize,
                _ => return -1,
            };

            let mut out_buf = vec![0u8; bytes_written];
            if memory
                .read(&mut caller, SCRATCH_OUTPUT_OFFSET, &mut out_buf)
                .is_err()
            {
                return -1;
            }

            let result_str = String::from_utf8(out_buf).unwrap_or_default();

            // Run WASM validators
            let validator_names: Vec<String> = {
                let world = caller.data().lock().unwrap();
                world.wasm_worldgen_validators.clone()
            };
            for v_name in &validator_names {
                if let Some(v_func) = caller.get_export(v_name).and_then(|e| e.into_func()) {
                    let _ = memory.write(&mut caller, SCRATCH_INPUT_OFFSET, result_str.as_bytes());
                    let mut v_results = [Val::I32(0)];
                    let _ = v_func.call(
                        &mut caller,
                        &[
                            Val::I32(SCRATCH_INPUT_OFFSET as i32),
                            Val::I32(result_str.len() as i32),
                        ],
                        &mut v_results,
                    );
                }
            }

            // Run WASM postprocessors (mutate result_str via output buffer)
            let mut final_result = result_str;
            let postprocessor_names: Vec<String> = {
                let world = caller.data().lock().unwrap();
                world.wasm_worldgen_postprocessors.clone()
            };
            for p_name in &postprocessor_names {
                if let Some(p_func) = caller.get_export(p_name).and_then(|e| e.into_func()) {
                    let _ =
                        memory.write(&mut caller, SCRATCH_INPUT_OFFSET, final_result.as_bytes());
                    let mut p_results = [Val::I32(0)];
                    if p_func
                        .call(
                            &mut caller,
                            &[
                                Val::I32(SCRATCH_INPUT_OFFSET as i32),
                                Val::I32(final_result.len() as i32),
                                Val::I32(SCRATCH_OUTPUT_OFFSET as i32),
                                Val::I32(SCRATCH_OUTPUT_MAX),
                            ],
                            &mut p_results,
                        )
                        .is_ok()
                        && let Val::I32(pw) = p_results[0]
                        && pw > 0
                    {
                        let mut p_buf = vec![0u8; pw as usize];
                        if memory
                            .read(&mut caller, SCRATCH_OUTPUT_OFFSET, &mut p_buf)
                            .is_ok()
                        {
                            let postprocessed = String::from_utf8(p_buf).unwrap_or_default();
                            if !postprocessed.is_empty() {
                                final_result = postprocessed;
                            }
                        }
                    }
                }
            }

            write_string_to_wasm(&mut caller, out_ptr, out_len, &final_result) as i32
        },
    )?;

    // --- Registration functions ---

    linker.func_wrap(
        "worldgen",
        "register_worldgen_plugin",
        move |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
              name_ptr: i32,
              name_len: i32,
              _type_ptr: i32,
              _type_len: i32|
              -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return -1,
            };

            // Check if the WASM guest exports mge_worldgen_generate
            let has_export = caller
                .get_export(EXPORT_WORLDGEN_GENERATE)
                .and_then(|e| e.into_func())
                .is_some();
            if !has_export {
                return -1;
            }

            // Store the plugin name in WasmWorld
            let mut world = caller.data().lock().unwrap();
            if !world.wasm_worldgen_plugins.contains(&name) {
                world.wasm_worldgen_plugins.push(name);
            }
            0
        },
    )?;

    linker.func_wrap(
        "worldgen",
        "register_worldgen_validator",
        move |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return -1,
            };

            // Check if the WASM guest exports mge_worldgen_validate
            let has_export = caller
                .get_export(EXPORT_WORLDGEN_VALIDATE)
                .and_then(|e| e.into_func())
                .is_some();
            if !has_export {
                return -1;
            }

            let mut world = caller.data().lock().unwrap();
            if !world.wasm_worldgen_validators.contains(&name) {
                world.wasm_worldgen_validators.push(name);
            }
            0
        },
    )?;

    linker.func_wrap(
        "worldgen",
        "register_worldgen_postprocessor",
        move |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, name_ptr: i32, name_len: i32| -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return -1,
            };

            // Check if the WASM guest exports mge_worldgen_postprocess
            let has_export = caller
                .get_export(EXPORT_WORLDGEN_POSTPROCESS)
                .and_then(|e| e.into_func())
                .is_some();
            if !has_export {
                return -1;
            }

            let mut world = caller.data().lock().unwrap();
            if !world.wasm_worldgen_postprocessors.contains(&name) {
                world.wasm_worldgen_postprocessors.push(name);
            }
            0
        },
    )?;

    Ok(())
}
