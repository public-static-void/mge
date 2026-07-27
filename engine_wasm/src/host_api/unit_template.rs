use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the unit template API.
///
/// Functions: load_unit_templates, register_unit_template, spawn_from_template,
/// get_unit_template, list_unit_templates.
pub fn register_unit_template_api(
    linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
) -> anyhow::Result<()> {
    // load_unit_templates(dir_ptr, dir_len)
    linker.func_wrap(
        "unit_template",
        "load_unit_templates",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, dir_ptr: i32, dir_len: i32| {
            let dir = read_wasm_string(&mut caller, dir_ptr, dir_len)
                .expect("Failed to read dir from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .load_templates_from_dir(&dir)
                .expect("Failed to load templates");
        },
    )?;

    // register_unit_template(json_ptr, json_len)
    linker.func_wrap(
        "unit_template",
        "register_unit_template",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, json_ptr: i32, json_len: i32| {
            let json = read_wasm_string(&mut caller, json_ptr, json_len)
                .expect("Failed to read template JSON from WASM memory");
            let mut world = caller.data().lock().unwrap();
            world
                .register_template_from_json(&json)
                .expect("Failed to register template");
        },
    )?;

    // spawn_from_template(name_ptr, name_len, overrides_ptr, overrides_len) -> i32 (entity_id)
    linker.func_wrap(
        "unit_template",
        "spawn_from_template",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         overrides_ptr: i32,
         overrides_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read template name from WASM memory");
            let overrides = if overrides_len > 0 {
                Some(
                    read_wasm_string(&mut caller, overrides_ptr, overrides_len)
                        .expect("Failed to read overrides from WASM memory"),
                )
            } else {
                None
            };
            let mut world = caller.data().lock().unwrap();
            match world.spawn_from_template(&name, overrides.as_deref()) {
                Ok(eid) => eid as i32,
                Err(e) => {
                    eprintln!("spawn_from_template error: {e}");
                    -1
                }
            }
        },
    )?;

    // get_unit_template(name_ptr, name_len, out_ptr, out_len) -> i32
    linker.func_wrap(
        "unit_template",
        "get_unit_template",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         name_ptr: i32,
         name_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let name = read_wasm_string(&mut caller, name_ptr, name_len)
                .expect("Failed to read template name from WASM memory");
            let json = {
                let world = caller.data().lock().unwrap();
                world.get_template_json(&name)
            };
            match json {
                Some(data) => write_string_to_wasm(&mut caller, out_ptr, out_len, &data) as i32,
                None => -1,
            }
        },
    )?;

    // list_unit_templates(out_ptr, out_len) -> i32
    linker.func_wrap(
        "unit_template",
        "list_unit_templates",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let names = {
                let world = caller.data().lock().unwrap();
                world.list_template_names()
            };
            let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json) as i32
        },
    )?;

    Ok(())
}
