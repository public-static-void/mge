use engine_core::ecs::world::World;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Holds the buffer and owner to keep the subscription alive for the test duration.
pub struct EventCapture {
    pub buffer: Arc<Mutex<VecDeque<serde_json::Value>>>,
    _owner: Arc<()>,
}

/// Subscribes to the event bus and collects all events into a buffer.
/// Keeps the owner alive to prevent the subscription from being dropped.
pub fn setup_event_capture(world: &mut World, event_name: &str) -> EventCapture {
    use engine_core::ecs::event::EventBus;
    let registry = &mut world.event_buses;

    let bus = registry
        .get_event_bus::<serde_json::Value>(event_name)
        .unwrap_or_else(|| {
            let new_bus = Arc::new(Mutex::new(EventBus::<serde_json::Value>::default()));
            registry.register_event_bus(event_name.to_string(), new_bus.clone());
            new_bus
        });

    let buffer = Arc::new(Mutex::new(VecDeque::new()));
    let owner = Arc::new(());
    let buffer_clone = buffer.clone();

    bus.lock().unwrap().subscribe_weak(&owner, move |event| {
        buffer_clone.lock().unwrap().push_back(event.clone());
    });

    EventCapture {
        buffer,
        _owner: owner,
    }
}
