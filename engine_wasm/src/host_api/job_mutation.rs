use crate::host_api::component::read_wasm_string;
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the job mutation API (set_job_field, update_job).
pub fn register_job_mutation_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "job_mutation",
        "set_job_field",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         job_id: i32,
         field_ptr: i32,
         field_len: i32,
         value_ptr: i32,
         value_len: i32|
         -> i32 {
            let field_name = match read_wasm_string(&mut caller, field_ptr, field_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let value_str = match read_wasm_string(&mut caller, value_ptr, value_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let value: JsonValue = match serde_json::from_str(&value_str) {
                Ok(v) => v,
                Err(_) => return -1,
            };

            let mut world = caller.data().lock().unwrap();
            let job_str = match world.get_component(job_id as u32, "Job") {
                Some(s) => s,
                None => return -1,
            };
            let mut job: JsonValue = match serde_json::from_str(&job_str) {
                Ok(j) => j,
                Err(_) => return -1,
            };

            job[&field_name] = value;

            let json_str = match serde_json::to_string(&job) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            match world.set_component(job_id as u32, "Job", &json_str) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_mutation",
        "update_job",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         job_id: i32,
         fields_ptr: i32,
         fields_len: i32|
         -> i32 {
            let data_str = match read_wasm_string(&mut caller, fields_ptr, fields_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let updates: JsonValue = match serde_json::from_str(&data_str) {
                Ok(v) => v,
                Err(_) => return -1,
            };

            let mut world = caller.data().lock().unwrap();
            let job_str = match world.get_component(job_id as u32, "Job") {
                Some(s) => s,
                None => return -1,
            };
            let mut job: JsonValue = match serde_json::from_str(&job_str) {
                Ok(j) => j,
                Err(_) => return -1,
            };

            if let Some(obj) = updates.as_object() {
                for (k, v) in obj {
                    job[k] = v.clone();
                }
            }

            let json_str = match serde_json::to_string(&job) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            match world.set_component(job_id as u32, "Job", &json_str) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    Ok(())
}
