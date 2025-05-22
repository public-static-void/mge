use crate::ecs::event::{EventBus, SubscriberId};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct EventBusRegistry {
    buses: HashMap<(TypeId, String), Arc<dyn Any + Send + Sync>>,
}

impl EventBusRegistry {
    pub fn new() -> Self {
        Self {
            buses: HashMap::new(),
        }
    }

    pub fn register_event_bus<T: 'static + Send + Sync>(
        &mut self,
        name: String,
        bus: Arc<Mutex<EventBus<T>>>,
    ) {
        self.buses
            .insert((TypeId::of::<T>(), name), bus as Arc<dyn Any + Send + Sync>);
    }

    pub fn get_event_bus<T: 'static + Send + Sync>(
        &self,
        name: &str,
    ) -> Option<Arc<Mutex<EventBus<T>>>> {
        self.buses
            .get(&(TypeId::of::<T>(), name.to_string()))
            .and_then(|arc_any| arc_any.clone().downcast::<Mutex<EventBus<T>>>().ok())
    }

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

    pub fn subscribe<T, F>(&self, name: &str, handler: F) -> Option<SubscriberId>
    where
        T: 'static + Send + Sync + Clone,
        F: Fn(&T) + Send + Sync + 'static,
    {
        self.get_event_bus::<T>(name)
            .map(|bus| bus.lock().unwrap().subscribe(handler))
    }

    pub fn unsubscribe<T>(&self, name: &str, id: SubscriberId) -> bool
    where
        T: 'static + Send + Sync + Clone,
    {
        self.get_event_bus::<T>(name)
            .map(|bus| bus.lock().unwrap().unsubscribe(id))
            .unwrap_or(false)
    }
}

impl Default for EventBusRegistry {
    fn default() -> Self {
        Self::new()
    }
}
