use engine_core::ecs::event::{EventBus, EventReader};
use mlua::{UserData, UserDataMethods};
use std::sync::{Arc, Mutex};

/// An event
#[derive(Clone)]
pub struct MyEvent(pub u32);

/// A Lua event bus
pub struct LuaEventBus {
    /// The inner event bus
    pub inner: Arc<Mutex<EventBus<MyEvent>>>,
}

impl LuaEventBus {
    /// Create a new event bus
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
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
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
