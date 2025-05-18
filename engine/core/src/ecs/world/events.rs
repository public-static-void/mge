use super::World;
use crate::ecs::event::EventBus;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};

impl World {
    pub fn send_event(&mut self, event_type: &str, payload: JsonValue) -> Result<(), String> {
        println!(
            "Rust: send_event called for type '{}' with payload {:?}",
            event_type, payload
        );
        let bus = self
            .event_buses
            .entry(event_type.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(EventBus::<JsonValue>::default())));
        bus.lock().unwrap().send(payload);
        Ok(())
    }

    pub fn get_event_bus(&self, event_type: &str) -> Option<Arc<Mutex<EventBus<JsonValue>>>> {
        self.event_buses.get(event_type).cloned()
    }

    pub fn get_or_create_event_bus(&mut self, event_type: &str) -> Arc<Mutex<EventBus<JsonValue>>> {
        self.event_buses
            .entry(event_type.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(EventBus::<JsonValue>::default())))
            .clone()
    }

    pub fn update_event_buses(&self) {
        for bus in self.event_buses.values() {
            bus.lock().unwrap().update();
        }
    }

    pub fn take_events(&mut self, event_type: &str) -> Vec<serde_json::Value> {
        if let Some(bus) = self.event_buses.get_mut(event_type) {
            let mut reader = crate::ecs::event::EventReader::default();
            let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
            bus.lock().unwrap().update();
            events
        } else {
            Vec::new()
        }
    }
}
