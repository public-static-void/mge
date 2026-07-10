use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the body part damage API (process_body_part_damage).
pub fn register_body_part_damage_api(
    linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
) -> anyhow::Result<()> {
    linker.func_wrap(
        "body_part_damage",
        "process_body_part_damage",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| {
            let mut world = caller.data().lock().unwrap();
            world.process_body_part_damage();
        },
    )?;

    Ok(())
}
