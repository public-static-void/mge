//! WASM host API for field-of-view functions.
//!
//! Functions registered under the `"wasm_fov"` namespace:
//! - `get_visible_cells(entity, out_ptr, out_len) -> i32`
//! - `is_visible(entity, x, y, z) -> i32`
//! - `set_sight(entity, range)`
//! - `get_sight(entity, out_ptr, out_len) -> i32`

use crate::host_api::component::write_string_to_wasm;
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::map::cell_key::CellKey;
use serde_json::json;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the FOV API (get_visible_cells, is_visible, set_sight, get_sight).
pub fn register_fov_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "wasm_fov",
        "get_visible_cells",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let cells_json: Vec<serde_json::Value> = {
                let world = caller.data().lock().unwrap();
                world
                    .get_visible_cells(entity)
                    .map(|cells| {
                        cells
                            .iter()
                            .map(|cell| match cell {
                                CellKey::Square { x, y, z } => {
                                    json!({"x": x, "y": y, "z": z})
                                }
                                CellKey::Hex { q, r, z } => {
                                    json!({"q": q, "r": r, "z": z})
                                }
                                CellKey::Province { id } => {
                                    json!({"id": id})
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            };
            if cells_json.is_empty() {
                -1
            } else {
                let json_str = serde_json::to_string(&cells_json)
                    .unwrap_or_else(|_| "[]".to_string());
                write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
            }
        },
    )?;

    linker.func_wrap(
        "wasm_fov",
        "is_visible",
         |caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
          entity: u32,
          x: i32,
          y: i32,
          z: i32|
         -> i32 {
            let visible = {
                let world = caller.data().lock().unwrap();
                let cell = CellKey::Square { x, y, z };
                world
                    .get_visible_cells(entity)
                    .map(|cells| cells.contains(&cell))
                    .unwrap_or(false)
            };
            if visible { 1 } else { 0 }
        },
    )?;

    linker.func_wrap(
        "wasm_fov",
        "set_sight",
         |caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
          entity: u32,
          range: i32| {
            let mut world = caller.data().lock().unwrap();
            let data = json!({
                "range": range,
            });
            world
                .set_component(entity, "Sight", &data.to_string())
                .expect("Failed to set Sight component");
        },
    )?;

    linker.func_wrap(
        "wasm_fov",
        "get_sight",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let world = caller.data().lock().unwrap();
                world.get_component(entity, "Sight")
            };
            match result {
                Some(json_str) => {
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
                }
                None => -1,
            }
        },
    )?;

    Ok(())
}
