use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the job board API (6 functions under the "job_board" import module).
///
/// Functions:
/// - `get_job_board(out_ptr, out_len) -> i32` — returns job board as JSON array
/// - `get_job_board_policy(out_ptr, out_len) -> i32` — returns current policy name
/// - `set_job_board_policy(policy_ptr, policy_len) -> i32` — sets policy (0 ok, -1 unknown)
/// - `get_job_priority(job_id, out_ptr, out_len) -> i32` — returns job priority or -1
/// - `set_job_priority(job_id, value) -> i32` — sets job priority (0 ok, -1 not found)
/// - `add_job_to_job_board(job_id) -> i32` — adds entity to job board
pub fn register_job_board_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "job_board",
        "get_job_board",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_job_board()
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    linker.func_wrap(
        "job_board",
        "get_job_board_policy",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let policy = {
                let world = caller.data().lock().unwrap();
                world.job_board_policy.clone()
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &policy) as i32
        },
    )?;

    linker.func_wrap(
        "job_board",
        "set_job_board_policy",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, policy_ptr: i32, policy_len: i32| -> i32 {
            let policy = read_wasm_string(&mut caller, policy_ptr, policy_len)
                .expect("Failed to read policy string from WASM memory");
            let mut world = caller.data().lock().unwrap();
            match world.set_job_board_policy(&policy) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_board",
        "get_job_priority",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         job_id: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let job_str = {
                let world = caller.data().lock().unwrap();
                world.get_component(job_id as u32, "Job")
            };
            match job_str {
                Some(data) => {
                    let job: JsonValue = serde_json::from_str(&data).unwrap_or_default();
                    let priority = job
                        .get("priority")
                        .cloned()
                        .unwrap_or(JsonValue::Number(0.into()));
                    let priority_str =
                        serde_json::to_string(&priority).unwrap_or_else(|_| "0".to_string());
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &priority_str) as i32
                }
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "job_board",
        "set_job_priority",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, job_id: i32, value: i64| -> i32 {
            let mut world = caller.data().lock().unwrap();
            let job_str = match world.get_component(job_id as u32, "Job") {
                Some(s) => s,
                None => return -1,
            };
            let mut job: JsonValue = match serde_json::from_str(&job_str) {
                Ok(j) => j,
                Err(_) => return -1,
            };
            job["priority"] = serde_json::json!(value);
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
        "job_board",
        "add_job_to_job_board",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, job_id: i32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            world.add_job_to_job_board(job_id as u32);
            0
        },
    )?;

    Ok(())
}
