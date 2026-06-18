use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the job cancel API (cancel_job).
pub fn register_job_cancel_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "job_cancel",
        "cancel_job",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, job_id: i32| -> i32 {
            let job_str = {
                let world = caller.data().lock().unwrap();
                world.get_component(job_id as u32, "Job")
            };
            let mut job = match job_str {
                Some(data) => match serde_json::from_str::<serde_json::Value>(&data) {
                    Ok(val) => val,
                    Err(_) => return -1,
                },
                None => return -1,
            };
            job["state"] = serde_json::json!("cancelled");
            let json_str = match serde_json::to_string(&job) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            {
                let mut world = caller.data().lock().unwrap();
                match world.set_component(job_id as u32, "Job", &json_str) {
                    Ok(_) => 0,
                    Err(_) => -1,
                }
            }
        },
    )?;
    Ok(())
}
