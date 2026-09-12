//! Weather API: get_weather, set_weather, get_weather_visibility_modifier.

use engine_core::ecs::world::{WeatherCondition, World};
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Register the weather API functions into the Lua globals table.
pub fn register_weather_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_weather() -> table { condition, intensity, duration_remaining }
    let w = world.clone();
    let get_weather = lua.create_function_mut(move |lua, ()| {
        let world = w.borrow();
        let weather = &world.weather;
        let tbl = lua.create_table()?;
        tbl.set("condition", weather.condition.as_str())?;
        tbl.set("intensity", weather.intensity)?;
        tbl.set("duration_remaining", weather.duration_remaining)?;
        Ok(tbl)
    })?;
    globals.set("get_weather", get_weather)?;

    // set_weather(condition, intensity, duration) -> nil
    // Unrecognized condition strings map to Clear; intensity is clamped to
    // [0.0, 1.0]; duration 0 forces a transition on the next tick.
    // Emits a "weather_changed" event like natural transitions (OQ4).
    let w = world.clone();
    let set_weather = lua.create_function_mut(
        move |_, (condition, intensity, duration): (String, f64, u32)| {
            let mut world = w.borrow_mut();
            let old_condition = world.weather.condition;
            let new_condition = WeatherCondition::from_name(&condition);
            let new_intensity = intensity.clamp(0.0, 1.0);
            world.weather.condition = new_condition;
            world.weather.intensity = new_intensity;
            world.weather.duration_remaining = duration;
            let _ = world.send_event(
                "weather_changed",
                serde_json::json!({
                    "old_condition": old_condition.as_str(),
                    "new_condition": new_condition.as_str(),
                    "intensity": new_intensity,
                }),
            );
            Ok(())
        },
    )?;
    globals.set("set_weather", set_weather)?;

    // get_weather_visibility_modifier() -> f64
    let w = world;
    let get_visibility_modifier = lua.create_function_mut(move |_, ()| {
        let world = w.borrow();
        Ok(world.visibility_modifier)
    })?;
    globals.set("get_weather_visibility_modifier", get_visibility_modifier)?;

    Ok(())
}
