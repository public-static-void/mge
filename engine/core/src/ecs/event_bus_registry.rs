use crate::ecs::event::EventBus;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct EventBusRegistry {
    buses: HashMap<String, Arc<Mutex<EventBus<JsonValue>>>>,
}

impl EventBusRegistry {
    pub fn new() -> Self {
        Self {
            buses: HashMap::new(),
        }
    }

    /// Register or overwrite an event bus.
    pub fn register_event_bus(&mut self, name: String, bus: Arc<Mutex<EventBus<JsonValue>>>) {
        self.buses.insert(name, bus);
    }

    /// Update (replace) an event bus if it exists.
    pub fn update_event_bus(
        &mut self,
        name: String,
        bus: Arc<Mutex<EventBus<JsonValue>>>,
    ) -> Result<(), String> {
        use std::collections::hash_map::Entry;
        match self.buses.entry(name) {
            Entry::Occupied(mut e) => {
                e.insert(bus);
                Ok(())
            }
            Entry::Vacant(e) => Err(format!("Event bus '{}' not found", e.key())),
        }
    }

    /// Unregister (remove) an event bus.
    pub fn unregister_event_bus(&mut self, name: &str) -> Result<(), String> {
        if self.buses.remove(name).is_some() {
            Ok(())
        } else {
            Err(format!("Event bus '{}' not found", name))
        }
    }

    /// Get an event bus by name.
    pub fn get_event_bus(&self, name: &str) -> Option<Arc<Mutex<EventBus<JsonValue>>>> {
        self.buses.get(name).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<Mutex<EventBus<JsonValue>>>> {
        self.buses.values()
    }
}

impl Default for EventBusRegistry {
    fn default() -> Self {
        Self::new()
    }
}
