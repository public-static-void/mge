//! Temperature API: get_temperature, set_temperature, humidity/pressure,
//! cell temperature.

use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the temperature API (get_temperature, set_temperature,
/// get_humidity, set_humidity, get_pressure, set_pressure,
/// get_cell_temperature).
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

    linker.func_wrap(
        "temperature",
        "get_humidity",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> f64 {
            let world = caller.data().lock().unwrap();
            world.get_humidity()
        },
    )?;

    linker.func_wrap(
        "temperature",
        "set_humidity",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, humidity: f64| -> i32 {
            let mut world = caller.data().lock().unwrap();
            world.set_humidity(humidity);
            0
        },
    )?;

    linker.func_wrap(
        "temperature",
        "get_pressure",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> f64 {
            let world = caller.data().lock().unwrap();
            world.get_pressure()
        },
    )?;

    linker.func_wrap(
        "temperature",
        "set_pressure",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, pressure: f64| -> i32 {
            let mut world = caller.data().lock().unwrap();
            world.set_pressure(pressure);
            0
        },
    )?;

    linker.func_wrap(
        "temperature",
        "get_cell_temperature",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, x: i32, y: i32, z: i32| -> f64 {
            let world = caller.data().lock().unwrap();
            world.get_cell_temperature(x, y, z)
        },
    )?;

    Ok(())
}
