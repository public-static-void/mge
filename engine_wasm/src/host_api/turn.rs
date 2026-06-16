use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the turn API (tick, get_turn).
pub fn register_turn_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "turn",
        "tick",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| {
            let mut world = caller.data().lock().unwrap();
            world.tick();
        },
    )?;

    linker.func_wrap(
        "turn",
        "get_turn",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> i32 {
            let world = caller.data().lock().unwrap();
            world.get_turn()
        },
    )?;

    Ok(())
}
