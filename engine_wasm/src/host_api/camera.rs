use crate::host_api::component::write_string_to_wasm;
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the camera API (set_camera, get_camera).
pub fn register_camera_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "camera",
        "set_camera",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, x: i32, y: i32, w: i32, h: i32| {
            let mut world = caller.data().lock().unwrap();
            world.set_camera(x, y, w, h);
            Ok(())
        },
    )?;

    linker.func_wrap(
        "camera",
        "get_camera",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_camera()
            };
            match json {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    Ok(())
}
