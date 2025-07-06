use crate::helpers::{json_to_lua_table, lua_error_from_any};
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the Lua event bus API functions.
pub fn register_event_bus_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // Register the event_bus object
    let event_bus = super::super::event_bus::LuaEventBus::new();
    globals.set("event_bus", event_bus)?;

    // send_event
    let world_send_event = world.clone();
    let send_event =
        lua.create_function_mut(move |lua, (event_type, payload): (String, String)| {
            let mut world = world_send_event.borrow_mut();
            let json_payload: JsonValue =
                serde_json::from_str(&payload).map_err(|e| lua_error_from_any(lua, e))?;
            world
                .send_event(&event_type, json_payload)
                .map_err(|e| lua_error_from_any(lua, e))
        })?;
    globals.set("send_event", send_event)?;

    // poll_event
    let event_readers = Rc::new(RefCell::new(std::collections::HashMap::<
        String,
        engine_core::ecs::event::EventReader,
    >::new()));
    let event_readers_for_closure = event_readers.clone();
    let world_for_closure = world.clone();
    let poll_event = lua.create_function_mut(move |lua, event_type: String| {
        let mut world = world_for_closure.borrow_mut();
        let bus = world.get_or_create_event_bus(&event_type);
        let mut readers = event_readers_for_closure.borrow_mut();
        let reader = readers.entry(event_type.clone()).or_default();
        let events: Vec<JsonValue> = reader.read_all(&*bus.lock().unwrap()).cloned().collect();
        let tbl = lua.create_table()?;
        for (i, val) in events.into_iter().enumerate() {
            tbl.set(i + 1, json_to_lua_table(lua, &val)?)?;
        }
        Ok(tbl)
    })?;
    globals.set("poll_event", poll_event)?;

    // update_event_buses
    let world_update_event_buses = world.clone();
    let update_event_buses = lua.create_function_mut(move |_, ()| {
        let world = world_update_event_buses.borrow();
        world.update_event_buses::<serde_json::Value>();
        Ok(())
    })?;
    globals.set("update_event_buses", update_event_buses)?;

    // poll_ecs_event
    let world_take_events = world.clone();
    let poll_ecs_event = lua.create_function_mut(move |lua, event_type: String| {
        let mut world = world_take_events.borrow_mut();
        let events = world.take_events(&event_type);
        let tbl = lua.create_table()?;
        for (i, val) in events.into_iter().enumerate() {
            tbl.set(i + 1, crate::helpers::json_to_lua_table(lua, &val)?)?;
        }
        Ok(tbl)
    })?;
    globals.set("poll_ecs_event", poll_ecs_event)?;

    Ok(())
}
