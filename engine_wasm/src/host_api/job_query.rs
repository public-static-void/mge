use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the job query API (8 host functions) under the "job_query" import module.
pub fn register_job_query_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "job_query",
        "list_jobs",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         include_terminal: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let world = caller.data().lock().unwrap();
                world.list_jobs(include_terminal != 0)
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &result) as i32
        },
    )?;

    linker.func_wrap(
        "job_query",
        "get_job",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         job_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_component(job_id, "Job")
            };
            match json {
                Some(data) => {
                    // Inject the id field into the job JSON
                    let mut job: JsonValue = serde_json::from_str(&data).unwrap_or(JsonValue::Null);
                    if let Some(obj) = job.as_object_mut() {
                        obj.insert("id".to_string(), serde_json::json!(job_id));
                    }
                    let enriched = serde_json::to_string(&job).unwrap_or_else(|_| data.clone());
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &enriched) as i32
                }
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_query",
        "find_jobs",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         criteria_ptr: i32,
         criteria_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let criteria = read_wasm_string(&mut caller, criteria_ptr, criteria_len)
                .expect("Failed to read filter JSON from WASM memory");
            let result = {
                let world = caller.data().lock().unwrap();
                world.find_jobs(&criteria)
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &result) as i32
        },
    )?;

    linker.func_wrap(
        "job_query",
        "advance_job_state",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, job_id: u32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            match world.advance_job_state(job_id) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_query",
        "get_job_children",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         job_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_component(job_id, "Job")
            };
            match json {
                Some(data) => {
                    let job: JsonValue = serde_json::from_str(&data).unwrap_or(JsonValue::Null);
                    let field = job.get("children").cloned().unwrap_or(JsonValue::Null);
                    let json_str =
                        serde_json::to_string(&field).unwrap_or_else(|_| "null".to_string());
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
                }
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_query",
        "set_job_children",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         job_id: u32,
         children_ptr: i32,
         children_len: i32|
         -> i32 {
            let children_json = read_wasm_string(&mut caller, children_ptr, children_len)
                .expect("Failed to read children JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            let job_str = match world.get_component(job_id, "Job") {
                Some(s) => s,
                None => return -1,
            };
            let mut job: JsonValue = serde_json::from_str(&job_str).unwrap_or(JsonValue::Null);
            let parsed: JsonValue = serde_json::from_str(&children_json).unwrap_or(JsonValue::Null);
            if let Some(obj) = job.as_object_mut() {
                obj.insert("children".to_string(), parsed);
            }
            let json_str = serde_json::to_string(&job).unwrap_or_default();
            let _ = world.set_component(job_id, "Job", &json_str);
            0
        },
    )?;

    linker.func_wrap(
        "job_query",
        "get_job_dependencies",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         job_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_component(job_id, "Job")
            };
            match json {
                Some(data) => {
                    let job: JsonValue = serde_json::from_str(&data).unwrap_or(JsonValue::Null);
                    let field = job.get("dependencies").cloned().unwrap_or(JsonValue::Null);
                    let json_str =
                        serde_json::to_string(&field).unwrap_or_else(|_| "null".to_string());
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
                }
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_query",
        "set_job_dependencies",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         job_id: u32,
         deps_ptr: i32,
         deps_len: i32|
         -> i32 {
            let deps_json = read_wasm_string(&mut caller, deps_ptr, deps_len)
                .expect("Failed to read dependencies JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            let job_str = match world.get_component(job_id, "Job") {
                Some(s) => s,
                None => return -1,
            };
            let mut job: JsonValue = serde_json::from_str(&job_str).unwrap_or(JsonValue::Null);
            let parsed: JsonValue = serde_json::from_str(&deps_json).unwrap_or(JsonValue::Null);
            if let Some(obj) = job.as_object_mut() {
                obj.insert("dependencies".to_string(), parsed);
            }
            let json_str = serde_json::to_string(&job).unwrap_or_default();
            let _ = world.set_component(job_id, "Job", &json_str);
            0
        },
    )?;

    Ok(())
}
