//! Noise API: emit_noise, get_noise_at, set_hearing, get_hearing.

use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the noise API functions into the Lua globals table.
pub fn register_noise_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // emit_noise(entity_id, intensity, radius) -> bool
    // Sets/updates the NoiseEmitter component; returns true on success.
    let w = world.clone();
    let emit_noise_fn =
        lua.create_function_mut(move |_, (entity_id, intensity, radius): (u32, f64, u32)| {
            let mut world = w.borrow_mut();
            let data = serde_json::json!({
                "intensity": intensity,
                "radius": radius,
                "active": true,
            });
            world
                .set_component(entity_id, "NoiseEmitter", data)
                .map_err(mlua::Error::external)?;
            Ok(true)
        })?;
    globals.set("emit_noise", emit_noise_fn)?;

    // get_noise_at(x, y, z) -> f64
    // Returns the noise level at a cell, or 0.0 when no noise was propagated.
    let w = world.clone();
    let get_noise_at_fn = lua.create_function_mut(move |_, (x, y, z): (i32, i32, i32)| {
        let world = w.borrow();
        let cell = CellKey::Square { x, y, z };
        Ok(world.get_noise_at(&cell).unwrap_or(0.0))
    })?;
    globals.set("get_noise_at", get_noise_at_fn)?;

    // set_hearing(entity_id, range, sensitivity?) -> bool
    // Sets/updates the Hearing component; sensitivity defaults to 1.0.
    let w = world.clone();
    let set_hearing_fn = lua.create_function_mut(
        move |_, (entity_id, range, sensitivity): (u32, u32, Option<f64>)| {
            let mut world = w.borrow_mut();
            let data = serde_json::json!({
                "range": range,
                "sensitivity": sensitivity.unwrap_or(1.0),
            });
            world
                .set_component(entity_id, "Hearing", data)
                .map_err(mlua::Error::external)?;
            Ok(true)
        },
    )?;
    globals.set("set_hearing", set_hearing_fn)?;

    // get_hearing(entity_id) -> table | nil
    let w = world;
    let get_hearing_fn = lua.create_function_mut(move |lua, entity_id: u32| {
        let world = w.borrow();
        let comp = world.get_component(entity_id, "Hearing");
        match comp {
            Some(data) => {
                let result = lua.create_table()?;
                if let Some(range) = data.get("range").and_then(|v| v.as_u64()) {
                    result.set("range", range)?;
                }
                if let Some(sensitivity) = data.get("sensitivity").and_then(|v| v.as_f64()) {
                    result.set("sensitivity", sensitivity)?;
                }
                if let Some(threshold) = data.get("threshold").and_then(|v| v.as_f64()) {
                    result.set("threshold", threshold)?;
                }
                Ok(Some(result))
            }
            None => Ok(None),
        }
    })?;
    globals.set("get_hearing", get_hearing_fn)?;

    Ok(())
}
