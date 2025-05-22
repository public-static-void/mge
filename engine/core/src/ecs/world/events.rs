use super::World;
use crate::ecs::event::EventBus;
use serde_json::Value as JsonValue;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

impl World {
    pub fn send_event(&mut self, event_type: &str, payload: JsonValue) -> Result<(), String> {
        println!(
            "Rust: send_event called for type '{}' with payload {:?}",
            event_type, payload
        );
        let bus = self
            .event_buses
            .get_event_bus(event_type)
            .unwrap_or_else(|| {
                let new_bus = Arc::new(Mutex::new(EventBus::<JsonValue>::default()));
                self.event_buses
                    .register_event_bus(event_type.to_string(), new_bus.clone());
                new_bus
            });
        bus.lock().unwrap().send(payload);
        Ok(())
    }

    pub fn get_event_bus(&self, event_type: &str) -> Option<Arc<Mutex<EventBus<JsonValue>>>> {
        self.event_buses.get_event_bus(event_type)
    }

    pub fn get_or_create_event_bus(&mut self, event_type: &str) -> Arc<Mutex<EventBus<JsonValue>>> {
        if let Some(bus) = self.event_buses.get_event_bus(event_type) {
            bus
        } else {
            let new_bus = Arc::new(Mutex::new(EventBus::<JsonValue>::default()));
            self.event_buses
                .register_event_bus(event_type.to_string(), new_bus.clone());
            new_bus
        }
    }

    pub fn update_event_buses(&self) {
        for bus in self.event_buses.iter() {
            bus.lock().unwrap().update();
        }
    }

    pub fn take_events(&mut self, event_type: &str) -> Vec<serde_json::Value> {
        if let Some(bus) = self.event_buses.get_event_bus(event_type) {
            let mut reader = crate::ecs::event::EventReader::default();
            let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
            bus.lock().unwrap().update();
            events
        } else {
            Vec::new()
        }
    }

    /// Emit an event of the given type. Events are delivered after the next update.
    pub fn emit_event(&mut self, event_type: &str, payload: JsonValue) {
        let queue = self
            .event_queues
            .entry(event_type.to_string())
            .or_insert_with(|| (VecDeque::new(), VecDeque::new()));
        queue.0.push_back(payload);
    }

    /// Process and consume all events of the given type, calling the handler for each.
    pub fn process_events<F: FnMut(&JsonValue)>(&mut self, event_type: &str, mut handler: F) {
        if let Some((_, read_queue)) = self.event_queues.get_mut(event_type) {
            while let Some(event) = read_queue.pop_front() {
                handler(&event);
            }
        }
    }

    /// Swap event buffers and clear the old read buffer.
    pub fn update_event_queues(&mut self) {
        for (_event_type, (write, read)) in self.event_queues.iter_mut() {
            std::mem::swap(write, read);
            write.clear();
        }
    }
}
