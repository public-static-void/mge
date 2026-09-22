//! Temperature API: get_temperature, set_temperature.

use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the temperature API (get_temperature, set_temperature).
pub fn register_temperature_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "temperature",
        "get_temperature",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> f64 {
            let world = caller.data().lock().unwrap();
            world.get_temperature()
        },
    )?;

    linker.func_wrap(
        "temperature",
        "set_temperature",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, ambient: f64| -> i32 {
            let mut world = caller.data().lock().unwrap();
            world.set_temperature(ambient);
            0
        },
    )?;

    Ok(())
}
