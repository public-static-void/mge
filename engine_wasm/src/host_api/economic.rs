use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::systems::job::reservation::resource_reservation::{
    ResourceReservationStatus, ResourceReservationSystem,
};
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the economic API (8 host functions).
pub fn register_economic_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "economic",
        "get_stockpile_resources",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_stockpile_resources(entity_id)
            };
            match result {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "economic",
        "get_production_job",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_production_job(entity_id)
            };
            match result {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "economic",
        "get_production_job_progress",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, entity_id: u32| -> f64 {
            let world = caller.data().lock().unwrap();
            world.get_production_job_progress(entity_id)
        },
    )?;

    linker.func_wrap(
        "economic",
        "set_production_job_progress",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, entity_id: u32, value: f64| {
            let mut world = caller.data().lock().unwrap();
            world.set_production_job_progress(entity_id, value);
        },
    )?;

    linker.func_wrap(
        "economic",
        "get_production_job_state",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let state = {
                let world = caller.data().lock().unwrap();
                world.get_production_job_state(entity_id)
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &state) as i32
        },
    )?;

    linker.func_wrap(
        "economic",
        "set_production_job_state",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         value_ptr: i32,
         value_len: i32| {
            let value = read_wasm_string(&mut caller, value_ptr, value_len)
                .expect("Failed to read state value from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world.set_production_job_state(entity_id, &value);
        },
    )?;

    linker.func_wrap(
        "economic",
        "modify_stockpile_resource",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         kind_ptr: i32,
         kind_len: i32,
         delta: f64| {
            let kind = read_wasm_string(&mut caller, kind_ptr, kind_len)
                .expect("Failed to read resource kind from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .modify_stockpile_resource(entity_id, &kind, delta)
                .expect("Failed to modify stockpile resource");
        },
    )?;

    linker.func_wrap(
        "economic",
        "get_job_resource_reservations",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity_id: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_job_resource_reservations(entity_id)
            };
            match result {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "economic",
        "reserve_job_resources",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, entity_id: i32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            let mut system = ResourceReservationSystem::new();
            system.run_reservation(&mut *world);
            let status = system.check_reservation_status(&*world, entity_id as u32);
            match status {
                ResourceReservationStatus::Reserved => 1,
                _ => 0,
            }
        },
    )?;

    linker.func_wrap(
        "economic",
        "release_job_resource_reservations",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>, entity_id: i32| -> i32 {
            let mut world = caller.data().lock().unwrap();
            let system = ResourceReservationSystem::new();
            system.release_reservation(&mut *world, entity_id as u32);
            0
        },
    )?;

    Ok(())
}
