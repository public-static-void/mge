//! Temperature API: get_temperature, set_temperature.

use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Register the temperature API functions into the Lua globals table.
pub fn register_temperature_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_temperature() -> f64 (current global ambient °C)
    let w = world.clone();
    let get_temperature = lua.create_function_mut(move |_, ()| {
        let world = w.borrow();
        Ok(world.get_temperature())
    })?;
    globals.set("get_temperature", get_temperature)?;

    // set_temperature(ambient) -> nil
    // Clamps to [-60, 60], holds the override, and emits a
    // "temperature_changed" event via the core method.
    let w = world;
    let set_temperature = lua.create_function_mut(move |_, ambient: f64| {
        let mut world = w.borrow_mut();
        world.set_temperature(ambient);
        Ok(())
    })?;
    globals.set("set_temperature", set_temperature)?;

    Ok(())
}
