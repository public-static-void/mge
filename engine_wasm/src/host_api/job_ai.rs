use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the Job AI API (3 functions under the "job_ai" import module).
///
/// Functions:
/// - `ai_assign_jobs(agent_id: i32) -> i32` — assigns highest-priority unassigned pending job
/// - `ai_query_jobs(entity_id: i32, out_ptr: i32, out_len: i32) -> i32` — returns jobs assigned to entity
/// - `ai_modify_job_assignment(entity_id: i32, job_id: i32, action_ptr: i32, action_len: i32) -> i32`
///   — merges changes JSON into Job component (handles `assigned_to: null` as explicit unassign)
pub fn register_job_ai_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "job_ai",
        "ai_assign_jobs",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, agent_id: i32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            match world.ai_assign_jobs(agent_id as u32) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_ai",
        "ai_query_jobs",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.ai_query_jobs(entity_id as u32)
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "job_ai",
        "ai_modify_job_assignment",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         _entity_id: i32,
         job_id: i32,
         action_ptr: i32,
         action_len: i32|
         -> i32 {
            let action = match read_wasm_string(&mut caller, action_ptr, action_len) {
                Ok(s) => s,
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

            // Parse changes JSON and merge into Job
            let changes: JsonValue = match serde_json::from_str(&action) {
                Ok(c) => c,
                Err(_) => return -1,
            };

            if let Some(obj) = changes.as_object() {
                for (k, v) in obj {
                    if v.is_null() {
                        // Explicit null: set field to null (used for unassigning)
                        job[k] = JsonValue::Null;
                    } else {
                        job[k] = v.clone();
                    }
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
