//! Temperature API: get_temperature, set_temperature, humidity/pressure.

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
    let w = world.clone();
    let set_temperature = lua.create_function_mut(move |_, ambient: f64| {
        let mut world = w.borrow_mut();
        world.set_temperature(ambient);
        Ok(())
    })?;
    globals.set("set_temperature", set_temperature)?;

    // get_humidity() -> f64 (current relative humidity in [0.0, 1.0])
    let w = world.clone();
    let get_humidity = lua.create_function_mut(move |_, ()| {
        let world = w.borrow();
        Ok(world.get_humidity())
    })?;
    globals.set("get_humidity", get_humidity)?;

    // set_humidity(v) -> nil (clamped to [0.0, 1.0] by the core method)
    let w = world.clone();
    let set_humidity = lua.create_function_mut(move |_, humidity: f64| {
        let mut world = w.borrow_mut();
        world.set_humidity(humidity);
        Ok(())
    })?;
    globals.set("set_humidity", set_humidity)?;

    // get_pressure() -> f64 (current atmospheric pressure in hPa)
    let w = world.clone();
    let get_pressure = lua.create_function_mut(move |_, ()| {
        let world = w.borrow();
        Ok(world.get_pressure())
    })?;
    globals.set("get_pressure", get_pressure)?;

    // set_pressure(v) -> nil (clamped to [900.0, 1100.0] by the core method)
    let w = world;
    let set_pressure = lua.create_function_mut(move |_, pressure: f64| {
        let mut world = w.borrow_mut();
        world.set_pressure(pressure);
        Ok(())
    })?;
    globals.set("set_pressure", set_pressure)?;

    Ok(())
}
