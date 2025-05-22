use std::collections::VecDeque;

pub type SubscriberId = usize;
pub type Subscriber<E> = (SubscriberId, Box<dyn Fn(&E) + Send + Sync>);

/// Double-buffered event bus for events of type E.
pub struct EventBus<E> {
    events: VecDeque<E>,
    last_events: VecDeque<E>,
    subscribers: Vec<Subscriber<E>>,
    next_subscriber_id: SubscriberId,
}

impl<E> Default for EventBus<E> {
    fn default() -> Self {
        Self {
            events: VecDeque::new(),
            last_events: VecDeque::new(),
            subscribers: Vec::new(),
            next_subscriber_id: 0,
        }
    }
}

impl<E: Clone + Send + Sync + 'static> EventBus<E> {
    /// Send (emit) an event.
    pub fn send(&mut self, event: E) {
        // Dispatch to subscribers immediately
        for (_, handler) in &self.subscribers {
            handler(&event);
        }
        self.events.push_back(event);
    }

    /// Subscribe a handler to this event bus.
    pub fn subscribe<F>(&mut self, handler: F) -> SubscriberId
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let id = self.next_subscriber_id;
        self.next_subscriber_id += 1;
        self.subscribers.push((id, Box::new(handler)));
        id
    }

    /// Unsubscribe a handler by its id.
    pub fn unsubscribe(&mut self, id: SubscriberId) -> bool {
        let prev_len = self.subscribers.len();
        self.subscribers.retain(|(sid, _)| *sid != id);
        prev_len != self.subscribers.len()
    }

    /// Advance the event bus to the next tick/frame.
    pub fn update(&mut self) {
        self.last_events = self.events.clone();
        self.events.clear();
    }

    pub fn last_events(&self) -> &VecDeque<E> {
        &self.last_events
    }

    pub fn try_recv(&mut self) -> Option<E> {
        self.events.pop_front()
    }
}

/// Reader for events, tracks which events have been read.
pub struct EventReader {
    last_index: usize,
}

impl EventReader {
    pub fn new() -> Self {
        Self { last_index: 0 }
    }
    pub fn read<'a, E>(&mut self, bus: &'a EventBus<E>) -> impl Iterator<Item = &'a E> {
        let events = &bus.last_events;
        let start = self.last_index.min(events.len());
        self.last_index = events.len();
        events.iter().skip(start)
    }
    pub fn read_all<'a, E>(&self, bus: &'a EventBus<E>) -> impl Iterator<Item = &'a E> {
        bus.last_events.iter()
    }
}

impl Default for EventReader {
    fn default() -> Self {
        Self::new()
    }
}
