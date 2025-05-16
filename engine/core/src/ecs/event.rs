use std::collections::VecDeque;

/// Double-buffered event bus for events of type E.
pub struct EventBus<E> {
    events: VecDeque<E>,
    last_events: VecDeque<E>,
}

impl<E> Default for EventBus<E> {
    fn default() -> Self {
        Self {
            events: VecDeque::new(),
            last_events: VecDeque::new(),
        }
    }
}

impl<E: Clone> EventBus<E> {
    /// Send (emit) an event.
    pub fn send(&mut self, event: E) {
        self.events.push_back(event);
    }

    /// Advance the event bus to the next tick/frame.
    /// Call this at the end of each ECS tick.
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
    /// Return only unread events since last read
    pub fn read<'a, E>(&mut self, bus: &'a EventBus<E>) -> impl Iterator<Item = &'a E> {
        let events = &bus.last_events;
        let start = self.last_index.min(events.len());
        self.last_index = events.len();
        events.iter().skip(start)
    }
    /// Return all events from last tick (for scripting)
    pub fn read_all<'a, E>(&self, bus: &'a EventBus<E>) -> impl Iterator<Item = &'a E> {
        bus.last_events.iter()
    }
}

impl Default for EventReader {
    fn default() -> Self {
        Self::new()
    }
}
