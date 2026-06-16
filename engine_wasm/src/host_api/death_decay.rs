use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the death/decay API (process_deaths, process_decay).
pub fn register_death_decay_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "death_decay",
        "process_deaths",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| {
            let mut world = caller.data().lock().unwrap();
            world.process_deaths();
        },
    )?;

    linker.func_wrap(
        "death_decay",
        "process_decay",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| {
            let mut world = caller.data().lock().unwrap();
            world.process_decay();
        },
    )?;

    Ok(())
}
