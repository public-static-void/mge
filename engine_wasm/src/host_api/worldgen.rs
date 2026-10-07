use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::worldgen::{
    GuestFailureStage, ThreadSafeScriptingWorldgenPlugin, ThreadSafeWorldgenPlugin,
    ThreadSafeWorldgenRegistry, WorldgenError,
};
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker, Val};

/// Known worldgen export names.
const EXPORT_WORLDGEN_GENERATE: &str = "mge_worldgen_generate";
const EXPORT_WORLDGEN_VALIDATE: &str = "mge_worldgen_validate";
const EXPORT_WORLDGEN_POSTPROCESS: &str = "mge_worldgen_postprocess";

/// Named scratch-buffer layout for guest worldgen calls (offsets into WASM
/// linear memory). Consolidates the former bare scratch-size literals.
pub mod wasm_layout {
    /// Scratch buffer offset for params passed to guest exports.
    pub const PARAM_SCRATCH_OFFSET: usize = 1024;
    /// Scratch buffer offset for output read back from guest exports.
    pub const RESULT_SCRATCH_OFFSET: usize = 4096;
    /// Maximum output buffer size handed to guest exports.
    pub const RESULT_SCRATCH_MAX: i32 = 4096;
}

/// Host-function code for duplicate registration (re-registering a name
/// without replace). Distinct from every [`GuestFailureStage`] code.
pub const WORLDGEN_DUPLICATE_CODE: i32 = -11;

/// Invoker delegate behind a guest adapter: maps params to a generated map
/// or a staged [`WorldgenError::Guest`] diagnostic.
pub type GuestInvoker = Arc<dyn Fn(&JsonValue) -> Result<JsonValue, WorldgenError> + Send + Sync>;

/// Scratch layout carried by a guest adapter: where params go in and where
/// results come out of guest linear memory.
#[derive(Debug, Clone, Copy)]
pub struct GuestScratchLayout {
    /// Offset of the params scratch buffer.
    pub params_offset: usize,
    /// Offset of the result scratch buffer.
    pub result_offset: usize,
    /// Maximum result buffer size.
    pub result_max: i32,
}

impl Default for GuestScratchLayout {
    fn default() -> Self {
        Self {
            params_offset: wasm_layout::PARAM_SCRATCH_OFFSET,
            result_offset: wasm_layout::RESULT_SCRATCH_OFFSET,
            result_max: wasm_layout::RESULT_SCRATCH_MAX,
        }
    }
}

/// Registry adapter for a WASM-guest worldgen plugin: holds the guest
/// identity (plugin name, export names, scratch layout) plus an invoker
/// delegate, and implements the thread-safe scripting plugin contract so the
/// guest registers as a real [`ThreadSafeWorldgenPlugin`] entry instead of a
/// string-list sidecar record.
///
/// Guest calls need a live guest context, so entries created by the
/// `register_worldgen_plugin` host function carry a host-routed invoker that
/// reports [`GuestFailureStage::RegistryInvoke`] when invoked directly through
/// the registry; the `invoke_worldgen_plugin` host dispatch (which owns the
/// live caller) runs the guest export instead. Delegates injected via
/// [`with_invoker`](Self::with_invoker) run inline, which is how the stage
/// diagnostics are exercised without a guest instance.
#[derive(Clone)]
pub struct WasmGuestWorldgenPlugin {
    plugin_name: String,
    generate_export: String,
    scratch: GuestScratchLayout,
    invoker: GuestInvoker,
}

impl WasmGuestWorldgenPlugin {
    /// Backend tag recorded on the registry entry.
    pub const BACKEND: &'static str = "wasm";

    /// Adapter for a host-registered guest: direct registry invokes explain
    /// the routing instead of failing silently.
    pub fn host_routed(name: &str) -> Self {
        let owned = name.to_owned();
        let detail_name = owned.clone();
        Self::with_invoker(&owned, move |_| {
            Err(WorldgenError::Guest {
                stage: GuestFailureStage::RegistryInvoke,
                detail: format!(
                    "guest plugin '{detail_name}' needs a live guest context; invoke it through the worldgen host dispatch"
                ),
            })
        })
    }

    /// Adapter with an explicit invoker delegate (tests, direct delegates).
    pub fn with_invoker(
        name: &str,
        invoker: impl Fn(&JsonValue) -> Result<JsonValue, WorldgenError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            plugin_name: name.to_owned(),
            generate_export: EXPORT_WORLDGEN_GENERATE.to_owned(),
            scratch: GuestScratchLayout::default(),
            invoker: Arc::new(invoker),
        }
    }

    /// Registered plugin name.
    pub fn name(&self) -> &str {
        &self.plugin_name
    }

    /// Generate-export name this guest is expected to provide.
    pub fn generate_export(&self) -> &str {
        &self.generate_export
    }

    /// Scratch layout this guest call uses.
    pub fn scratch(&self) -> GuestScratchLayout {
        self.scratch
    }

    /// Wrap this adapter as a registry entry.
    pub fn into_registry_entry(self) -> ThreadSafeWorldgenPlugin {
        ThreadSafeWorldgenPlugin::ThreadSafeScripting {
            name: self.plugin_name.clone(),
            backend: Self::BACKEND.to_owned(),
            opaque: Box::new(self),
        }
    }
}

impl ThreadSafeScriptingWorldgenPlugin for WasmGuestWorldgenPlugin {
    fn invoke(&self, params: &JsonValue) -> Result<JsonValue, Box<dyn std::error::Error>> {
        (self.invoker)(params).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn backend(&self) -> &str {
        Self::BACKEND
    }
}

/// Run the guest generate export for a registered guest entry using the live
/// caller, then its validators and postprocessors. Every former silent `-1`
/// stage returns a [`WorldgenError::Guest`] naming the stage.
///
/// Hook verdict contract: a validator returning `0` accepts, any other code
/// rejects; a postprocessor returning `0` keeps the input, a positive count
/// replaces it with that many output bytes, a negative code reports failure.
fn invoke_guest_worldgen(
    caller: &mut Caller<'_, Arc<Mutex<WasmWorld>>>,
    plugin_name: &str,
    params: &JsonValue,
) -> Result<String, WorldgenError> {
    let guest_error =
        |stage: GuestFailureStage, detail: String| WorldgenError::Guest { stage, detail };

    let generate_func = caller
        .get_export(EXPORT_WORLDGEN_GENERATE)
        .and_then(|e| e.into_func())
        .ok_or_else(|| {
            guest_error(
                GuestFailureStage::NoExport,
                format!("guest plugin '{plugin_name}' exports no {EXPORT_WORLDGEN_GENERATE}"),
            )
        })?;
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| {
            guest_error(
                GuestFailureStage::NoMemory,
                format!("guest plugin '{plugin_name}' exposes no linear memory"),
            )
        })?;

    let params_json = serde_json::to_string(params).unwrap_or_default();
    memory
        .write(
            &mut *caller,
            wasm_layout::PARAM_SCRATCH_OFFSET,
            params_json.as_bytes(),
        )
        .map_err(|e| {
            guest_error(
                GuestFailureStage::WriteParams,
                format!("guest plugin '{plugin_name}': params write failed: {e}"),
            )
        })?;

    let mut results = [Val::I32(0)];
    generate_func
        .call(
            &mut *caller,
            &[
                Val::I32(wasm_layout::PARAM_SCRATCH_OFFSET as i32),
                Val::I32(params_json.len() as i32),
                Val::I32(wasm_layout::RESULT_SCRATCH_OFFSET as i32),
                Val::I32(wasm_layout::RESULT_SCRATCH_MAX),
            ],
            &mut results,
        )
        .map_err(|e| {
            guest_error(
                GuestFailureStage::Call,
                format!("guest plugin '{plugin_name}': generate call trapped: {e}"),
            )
        })?;

    let bytes_written = match results[0] {
        Val::I32(n) if n >= 0 => n as usize,
        Val::I32(n) => {
            return Err(guest_error(
                GuestFailureStage::Call,
                format!("guest plugin '{plugin_name}': generate reported error code {n}"),
            ));
        }
        _ => {
            return Err(guest_error(
                GuestFailureStage::Call,
                format!("guest plugin '{plugin_name}': generate returned non-i32"),
            ));
        }
    };

    let mut out_buf = vec![0u8; bytes_written];
    memory
        .read(
            &mut *caller,
            wasm_layout::RESULT_SCRATCH_OFFSET,
            &mut out_buf,
        )
        .map_err(|e| {
            guest_error(
                GuestFailureStage::ReadResult,
                format!("guest plugin '{plugin_name}': result read failed: {e}"),
            )
        })?;
    let result_str = String::from_utf8(out_buf).map_err(|e| {
        guest_error(
            GuestFailureStage::ParseResult,
            format!("guest plugin '{plugin_name}': result is not UTF-8: {e}"),
        )
    })?;
    serde_json::from_str::<JsonValue>(&result_str).map_err(|e| {
        guest_error(
            GuestFailureStage::ParseResult,
            format!("guest plugin '{plugin_name}': result is not JSON: {e}"),
        )
    })?;

    // Guest validators: verdicts are honored (0 accepts, any other code
    // rejects) instead of discarded.
    let validator_names: Vec<String> = {
        caller
            .data()
            .lock()
            .unwrap()
            .wasm_worldgen_validators
            .clone()
    };
    for v_name in &validator_names {
        let Some(v_func) = caller.get_export(v_name).and_then(|e| e.into_func()) else {
            continue;
        };
        memory
            .write(
                &mut *caller,
                wasm_layout::PARAM_SCRATCH_OFFSET,
                result_str.as_bytes(),
            )
            .map_err(|e| {
                guest_error(
                    GuestFailureStage::Validate,
                    format!("guest validator '{v_name}': scratch write failed: {e}"),
                )
            })?;
        let mut v_results = [Val::I32(0)];
        v_func
            .call(
                &mut *caller,
                &[
                    Val::I32(wasm_layout::PARAM_SCRATCH_OFFSET as i32),
                    Val::I32(result_str.len() as i32),
                ],
                &mut v_results,
            )
            .map_err(|e| {
                guest_error(
                    GuestFailureStage::Validate,
                    format!("guest validator '{v_name}': call trapped: {e}"),
                )
            })?;
        match v_results[0] {
            Val::I32(0) => {}
            Val::I32(code) => {
                return Err(guest_error(
                    GuestFailureStage::Validate,
                    format!("guest validator '{v_name}' rejected the map (code {code})"),
                ));
            }
            _ => {
                return Err(guest_error(
                    GuestFailureStage::Validate,
                    format!("guest validator '{v_name}' returned non-i32 verdict"),
                ));
            }
        }
    }

    // Guest postprocessors: the byte-count contract is explicit (0 keeps the
    // input, positive replaces it, negative reports failure).
    let mut final_result = result_str;
    let postprocessor_names: Vec<String> = {
        caller
            .data()
            .lock()
            .unwrap()
            .wasm_worldgen_postprocessors
            .clone()
    };
    for p_name in &postprocessor_names {
        let Some(p_func) = caller.get_export(p_name).and_then(|e| e.into_func()) else {
            continue;
        };
        memory
            .write(
                &mut *caller,
                wasm_layout::PARAM_SCRATCH_OFFSET,
                final_result.as_bytes(),
            )
            .map_err(|e| {
                guest_error(
                    GuestFailureStage::Postprocess,
                    format!("guest postprocessor '{p_name}': scratch write failed: {e}"),
                )
            })?;
        let mut p_results = [Val::I32(0)];
        p_func
            .call(
                &mut *caller,
                &[
                    Val::I32(wasm_layout::PARAM_SCRATCH_OFFSET as i32),
                    Val::I32(final_result.len() as i32),
                    Val::I32(wasm_layout::RESULT_SCRATCH_OFFSET as i32),
                    Val::I32(wasm_layout::RESULT_SCRATCH_MAX),
                ],
                &mut p_results,
            )
            .map_err(|e| {
                guest_error(
                    GuestFailureStage::Postprocess,
                    format!("guest postprocessor '{p_name}': call trapped: {e}"),
                )
            })?;
        match p_results[0] {
            Val::I32(0) => {}
            Val::I32(pw) if pw > 0 => {
                let mut p_buf = vec![0u8; pw as usize];
                memory
                    .read(&mut *caller, wasm_layout::RESULT_SCRATCH_OFFSET, &mut p_buf)
                    .map_err(|e| {
                        guest_error(
                            GuestFailureStage::Postprocess,
                            format!("guest postprocessor '{p_name}': result read failed: {e}"),
                        )
                    })?;
                let postprocessed = String::from_utf8(p_buf).map_err(|e| {
                    guest_error(
                        GuestFailureStage::Postprocess,
                        format!("guest postprocessor '{p_name}': result is not UTF-8: {e}"),
                    )
                })?;
                if !postprocessed.is_empty() {
                    final_result = postprocessed;
                }
            }
            Val::I32(pw) => {
                return Err(guest_error(
                    GuestFailureStage::Postprocess,
                    format!("guest postprocessor '{p_name}' reported error code {pw}"),
                ));
            }
            _ => {
                return Err(guest_error(
                    GuestFailureStage::Postprocess,
                    format!("guest postprocessor '{p_name}' returned non-i32"),
                ));
            }
        }
    }

    Ok(final_result)
}

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
            // Guest plugins are real registry entries, so the registry is
            // the single source of names.
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
            let params: JsonValue = serde_json::from_str(&params_str).unwrap_or(JsonValue::Null);

            // Registry-first dispatch: registered guest entries run through
            // the guest export call with the live caller, so every failure
            // stage reports a distinguishable code. Unknown names keep the
            // legacy `-1` contract; other registry errors map the same way.
            let is_guest_entry = reg2.lock().unwrap().entry_backend(&name).as_deref()
                == Some(WasmGuestWorldgenPlugin::BACKEND);
            if is_guest_entry {
                return match invoke_guest_worldgen(&mut caller, &name, &params) {
                    Ok(json) => write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32,
                    Err(e) => e.guest_code(),
                };
            }

            match reg2.lock().unwrap().invoke(&name, &params) {
                Ok(map) => {
                    let json = serde_json::to_string(&map).unwrap_or_default();
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
                }
                Err(e) => e.guest_code(),
            }
        },
    )?;

    // --- Registration functions ---

    let reg3 = worldgen_registry.clone();
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

            // The guest must export the generate function; otherwise nothing
            // is recorded and the caller learns the exact stage.
            let has_export = caller
                .get_export(EXPORT_WORLDGEN_GENERATE)
                .and_then(|e| e.into_func())
                .is_some();
            if !has_export {
                return GuestFailureStage::NoExport.code();
            }

            // Record the guest as a real registry entry (subject to the
            // duplicate policy), not a string-list sidecar record.
            let entry = WasmGuestWorldgenPlugin::host_routed(&name).into_registry_entry();
            match reg3.lock().unwrap().register(entry) {
                Ok(()) => 0,
                Err(_) => WORLDGEN_DUPLICATE_CODE,
            }
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
