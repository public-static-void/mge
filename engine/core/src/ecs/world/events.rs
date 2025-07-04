use super::World;
use crate::ecs::event::{EventBus, SubscriberId};
use serde_json::Value as JsonValue;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

impl World {
    /// Send an event to the given event bus
    pub fn send_event(&mut self, event_type: &str, payload: JsonValue) -> Result<(), String> {
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

    /// Get an event bus by name
    pub fn get_event_bus<T: 'static + Send + Sync>(
        &self,
        name: &str,
    ) -> Option<Arc<Mutex<EventBus<T>>>> {
        self.event_buses.get_event_bus::<T>(name)
    }

    /// Get or create an event bus
    pub fn get_or_create_event_bus<T: 'static + Send + Sync>(
        &mut self,
        name: &str,
    ) -> Arc<Mutex<EventBus<T>>> {
        if let Some(bus) = self.event_buses.get_event_bus::<T>(name) {
            bus
        } else {
            self.register_event_bus::<T>(name)
        }
    }

    /// Update all event buses
    pub fn update_event_buses<T: 'static + Send + Sync + Clone>(&self) {
        self.event_buses.update_event_buses::<T>();
    }

    /// Get all events of the given type
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

    /// Register a new event bus
    pub fn register_event_bus<T: 'static + Send + Sync>(
        &mut self,
        name: &str,
    ) -> Arc<Mutex<EventBus<T>>> {
        let bus = Arc::new(Mutex::new(EventBus::<T>::default()));
        self.event_buses
            .register_event_bus::<T>(name.to_string(), bus.clone());
        bus
    }

    /// Subscribe to an event
    pub fn subscribe<T, F>(&self, name: &str, handler: F) -> Option<SubscriberId>
    where
        T: 'static + Send + Sync + Clone,
        F: Fn(&T) + Send + Sync + 'static,
    {
        self.event_buses.subscribe::<T, F>(name, handler)
    }

    /// Unsubscribe from an event
    pub fn unsubscribe<T>(&self, name: &str, id: SubscriberId) -> bool
    where
        T: 'static + Send + Sync + Clone,
    {
        self.event_buses.unsubscribe::<T>(name, id)
    }

    /// List all event buses
    pub fn list_event_buses(&self) -> Vec<crate::ecs::event_bus_registry::EventBusInfo> {
        self.event_buses.list_buses()
    }

    /// List all event bus names
    pub fn list_event_bus_names(&self) -> Vec<String> {
        self.event_buses.list_bus_names()
    }

    /// List all event bus types and names
    pub fn list_event_bus_types_and_names(&self) -> Vec<(String, String)> {
        self.event_buses.list_bus_types_and_names()
    }

    /// Get the number of subscribers to the given event
    pub fn event_bus_subscriber_count<T: 'static + Send + Sync>(
        &self,
        name: &str,
    ) -> Option<usize> {
        self.event_buses.subscriber_count::<T>(name)
    }

    /// Register a new effect handler
    pub fn register_effect_handler<F>(&mut self, action: &str, handler: F)
    where
        F: Fn(&mut World, u32, &serde_json::Value) + Send + Sync + 'static,
    {
        self.effect_processor_registry
            .as_ref()
            .expect("EffectProcessorRegistry missing")
            .lock()
            .unwrap()
            .register_handler(action, handler);
    }

    /// Drains all events of the given type and name, advancing the event bus.
    /// This is the canonical way to get all events for a tick/frame in both production and tests.
    pub fn drain_events<T: 'static + Send + Sync + Clone>(&mut self, event_name: &str) -> Vec<T> {
        if let Some(bus) = self.get_event_bus::<T>(event_name) {
            let reader = crate::ecs::event::EventReader::default();
            let events: Vec<_> = reader.read_all(&*bus.lock().unwrap()).cloned().collect();
            bus.lock().unwrap().update();
            events
        } else {
            Vec::new()
        }
    }
}
