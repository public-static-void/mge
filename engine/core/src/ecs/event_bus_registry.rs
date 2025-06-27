use crate::ecs::event::{EventBus, SubscriberId};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Metadata about a registered event bus.
pub struct EventBusInfo {
    /// The type ID of the event bus.
    pub type_id: TypeId,
    /// The type name (best effort).
    pub type_name: &'static str,
    /// The name of the event bus.
    pub name: String,
    /// The number of subscribers.
    pub subscriber_count: usize,
}

/// Registry for event buses of various types.
pub struct EventBusRegistry {
    buses: HashMap<(TypeId, String), Arc<dyn Any + Send + Sync>>,
}

impl EventBusRegistry {
    /// Creates a new, empty event bus registry.
    pub fn new() -> Self {
        Self {
            buses: HashMap::new(),
        }
    }

    /// Registers an event bus of type T with the given name.
    pub fn register_event_bus<T: 'static + Send + Sync>(
        &mut self,
        name: String,
        bus: Arc<Mutex<EventBus<T>>>,
    ) {
        self.buses
            .insert((TypeId::of::<T>(), name), bus as Arc<dyn Any + Send + Sync>);
    }

    /// Retrieves an event bus by type and name.
    pub fn get_event_bus<T: 'static + Send + Sync>(
        &self,
        name: &str,
    ) -> Option<Arc<Mutex<EventBus<T>>>> {
        self.buses
            .get(&(TypeId::of::<T>(), name.to_string()))
            .and_then(|arc_any| arc_any.clone().downcast::<Mutex<EventBus<T>>>().ok())
    }

    /// Unregisters an event bus by type and name.
    pub fn unregister_event_bus<T: 'static + Send + Sync>(&mut self, name: &str) -> bool {
        self.buses
            .remove(&(TypeId::of::<T>(), name.to_string()))
            .is_some()
    }

    /// Iterate over all event buses of type T.
    pub fn iter<T: 'static + Send + Sync>(
        &self,
    ) -> impl Iterator<Item = (&String, Arc<Mutex<EventBus<T>>>)> {
        self.buses.iter().filter_map(|((type_id, name), arc_any)| {
            if *type_id == TypeId::of::<T>() {
                arc_any
                    .clone()
                    .downcast::<Mutex<EventBus<T>>>()
                    .ok()
                    .map(|arc| (name, arc))
            } else {
                None
            }
        })
    }

    /// Update all event buses of type T.
    pub fn update_event_buses<T: 'static + Send + Sync + Clone>(&self) {
        for (_name, bus) in self.iter::<T>() {
            bus.lock().unwrap().update();
        }
    }

    /// Subscribe to an event bus by type and name.
    pub fn subscribe<T, F>(&self, name: &str, handler: F) -> Option<SubscriberId>
    where
        T: 'static + Send + Sync + Clone,
        F: Fn(&T) + Send + Sync + 'static,
    {
        self.get_event_bus::<T>(name)
            .map(|bus| bus.lock().unwrap().subscribe(handler))
    }

    /// Unsubscribe from an event bus by type and name.
    pub fn unsubscribe<T>(&self, name: &str, id: SubscriberId) -> bool
    where
        T: 'static + Send + Sync + Clone,
    {
        self.get_event_bus::<T>(name)
            .map(|bus| bus.lock().unwrap().unsubscribe(id))
            .unwrap_or(false)
    }

    /// List all registered event buses with metadata.
    pub fn list_buses(&self) -> Vec<EventBusInfo> {
        self.buses
            .iter()
            .map(|((type_id, name), arc_any)| {
                // Try to downcast to EventBus of any type to get subscriber count.
                // We'll use type_name for a best-effort type display.
                let type_name = "<unknown>";
                let subscriber_count = {
                    // Try to get subscriber count for common types (JsonValue, etc.)
                    // If you want to support more types, add them here.
                    if let Ok(bus) = arc_any
                        .clone()
                        .downcast::<Mutex<EventBus<serde_json::Value>>>()
                    {
                        bus.lock().unwrap().subscriber_count()
                    } else {
                        // For other types, we can't get subscriber count without type info.
                        0
                    }
                };
                EventBusInfo {
                    type_id: *type_id,
                    type_name,
                    name: name.clone(),
                    subscriber_count,
                }
            })
            .collect()
    }

    /// List all event bus names.
    pub fn list_bus_names(&self) -> Vec<String> {
        self.buses.keys().map(|(_, name)| name.clone()).collect()
    }

    /// List all event bus type_ids.
    pub fn list_bus_type_ids(&self) -> Vec<TypeId> {
        self.buses.keys().map(|(type_id, _)| *type_id).collect()
    }

    /// List all (type_name, name) pairs for all buses.
    pub fn list_bus_types_and_names(&self) -> Vec<(String, String)> {
        self.buses
            .iter()
            .map(|((type_id, name), _)| {
                (
                    format!("{type_id:?}"), // Or use a registry of type_name if you maintain one
                    name.clone(),
                )
            })
            .collect()
    }

    /// Get subscriber count for a bus of type T.
    pub fn subscriber_count<T: 'static + Send + Sync>(&self, name: &str) -> Option<usize> {
        self.get_event_bus::<T>(name)
            .map(|bus| bus.lock().unwrap().subscriber_count())
    }
}

impl Default for EventBusRegistry {
    fn default() -> Self {
        Self::new()
    }
}
