use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the job system API (assign_job, get_job_types, register_job_type, get_job_type_metadata).
pub fn register_job_system_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "job_system",
        "assign_job",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: i32,
         type_ptr: i32,
         type_len: i32,
         data_ptr: i32,
         data_len: i32|
         -> i32 {
            // Read job type string from WASM memory
            let job_type = match read_wasm_string(&mut caller, type_ptr, type_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };

            // Construct base Job JSON with id, job_type, state, progress
            let mut job = serde_json::json!({
                "id": entity_id,
                "job_type": job_type,
                "state": "pending",
                "progress": 0.0
            });

            // Merge optional fields JSON if data_len > 0
            if data_len > 0
                && let Ok(data_str) = read_wasm_string(&mut caller, data_ptr, data_len)
                && let Ok(fields_val) = serde_json::from_str::<serde_json::Value>(&data_str)
                && let Some(obj) = fields_val.as_object()
            {
                for (k, v) in obj {
                    job[k] = v.clone();
                }
            }

            let json_str = serde_json::to_string(&job).unwrap_or_default();
            let mut world = caller.data().lock().unwrap();
            match world.set_component(entity_id as u32, "Job", &json_str) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_system",
        "get_job_types",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let names = {
                let world = caller.data().lock().unwrap();
                world.get_job_type_names()
            };
            let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "job_system",
        "register_job_type",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         metadata_ptr: i32,
         metadata_len: i32|
         -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return -1,
            };
            let metadata = match read_wasm_string(&mut caller, metadata_ptr, metadata_len) {
                Ok(m) => m,
                Err(_) => return -1,
            };

            // Export discovery: verify the calling module exports mge_job_handler_<sanitized_name>
            let sanitized = sanitize_for_export(&name);
            let export_name = format!("mge_job_handler_{}", sanitized);
            if caller.get_export(&export_name).is_none() {
                return -1;
            }

            let mut world = caller.data().lock().unwrap();
            match world.register_job_type(&name, &metadata) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_system",
        "get_job_type_metadata",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let name = match read_wasm_string(&mut caller, name_ptr, name_len) {
                Ok(n) => n,
                Err(_) => return -1,
            };
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_job_type_metadata(&name)
            };
            match result {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    Ok(())
}

/// Sanitizes a job type name for use as a WASM export name.
/// Replaces all non-alphanumeric characters (except underscore) with underscore.
/// Lowercases the result.
fn sanitize_for_export(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}
