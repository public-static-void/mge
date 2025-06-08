use mlua::{Function as LuaFunction, Lua};
use std::collections::HashMap;
use std::sync::Mutex;

type WidgetId = u64;

#[derive(Default)]
pub struct LuaCallbackRegistry {
    // (widget_id, event_name) -> LuaFunction
    callbacks: HashMap<(WidgetId, String), LuaFunction>,
}

impl LuaCallbackRegistry {
    pub fn set_callback(&mut self, widget_id: WidgetId, event: &str, func: LuaFunction) {
        self.callbacks.insert((widget_id, event.to_string()), func);
    }

    pub fn remove_callback(&mut self, widget_id: WidgetId, event: &str) {
        self.callbacks.remove(&(widget_id, event.to_string()));
    }

    pub fn get_callback(&self, widget_id: WidgetId, event: &str) -> Option<&LuaFunction> {
        self.callbacks.get(&(widget_id, event.to_string()))
    }
}

// Global, but only accessed from main thread/event loop!
pub static LUA_CALLBACK_REGISTRY: Mutex<LuaCallbackRegistry> =
    Mutex::new(LuaCallbackRegistry::default());
