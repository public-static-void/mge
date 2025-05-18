use crate::ecs::event::{EventBus, EventReader};
use crate::scripting::helpers::json_to_lua_table;
use crate::scripting::world::World;
use mlua::{Lua, Result as LuaResult, Table, UserData, UserDataMethods};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MyEvent(pub u32);

pub struct LuaEventBus {
    pub inner: Arc<Mutex<EventBus<MyEvent>>>,
}

impl LuaEventBus {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(EventBus::default())),
        }
    }
}

impl Default for LuaEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl UserData for LuaEventBus {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("send", |_, this, value: u32| {
            this.inner.lock().unwrap().send(MyEvent(value));
            Ok(())
        });

        methods.add_method("poll", |_, this, ()| {
            let mut reader = EventReader::default();
            let bus = this.inner.lock().unwrap();
            let events: Vec<u32> = reader.read(&*bus).map(|e| e.0).collect();
            Ok(events)
        });

        methods.add_method_mut("update", |_, this, ()| {
            this.inner.lock().unwrap().update();
            Ok(())
        });
    }
}

pub fn register_event_bus_and_globals(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // Register the event_bus object
    let event_bus = LuaEventBus::new();
    globals.set("event_bus", event_bus)?;

    // send_event
    let world_send_event = world.clone();
    let send_event =
        lua.create_function_mut(move |_, (event_type, payload): (String, String)| {
            let mut world = world_send_event.borrow_mut();
            let json_payload: JsonValue =
                serde_json::from_str(&payload).map_err(mlua::Error::external)?;
            world
                .send_event(&event_type, json_payload)
                .map_err(mlua::Error::external)
        })?;
    globals.set("send_event", send_event)?;

    // poll_event
    let event_readers = Rc::new(RefCell::new(std::collections::HashMap::<
        String,
        crate::ecs::event::EventReader,
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
        world.update_event_buses();
        Ok(())
    })?;
    globals.set("update_event_buses", update_event_buses)?;

    Ok(())
}
