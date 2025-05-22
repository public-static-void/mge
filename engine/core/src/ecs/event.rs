use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Weak};

pub type SubscriberId = usize;
pub type FilterFn<E> = Box<dyn Fn(&E) -> bool + Send + Sync>;
pub type MapFn<E, U> = Box<dyn Fn(&E) -> Option<U> + Send + Sync>;

pub struct Subscriber<E> {
    pub id: SubscriberId,
    pub handler: Box<dyn Fn(&E) + Send + Sync>,
    pub once: bool,
    pub weak_owner: Option<Weak<()>>,
}

/// Double-buffered event bus for events of type E.
/// Only the event buffers are serializable; subscribers are always runtime-only.
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

impl<E> EventBus<E> {
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Serialize the event bus (events and last_events only).
    pub fn serialize_events<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        E: Serialize,
        S: serde::Serializer,
    {
        (&self.events, &self.last_events).serialize(serializer)
    }

    /// Deserialize into a new event bus (subscribers will be empty).
    pub fn deserialize_events<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        E: Deserialize<'de>,
        D: serde::Deserializer<'de>,
    {
        let (events, last_events) = <(VecDeque<E>, VecDeque<E>)>::deserialize(deserializer)?;
        Ok(EventBus {
            events,
            last_events,
            subscribers: Vec::new(),
            next_subscriber_id: 0,
        })
    }

    pub fn set_events(&mut self, events: std::collections::VecDeque<E>) {
        self.events = events;
    }

    pub fn set_last_events(&mut self, last_events: std::collections::VecDeque<E>) {
        self.last_events = last_events;
    }
}

impl<E: Clone + Send + Sync + 'static> EventBus<E> {
    /// Send (emit) an event.
    pub fn send(&mut self, event: E) {
        let mut to_remove = Vec::new();

        for sub in &self.subscribers {
            // Remove if weak_owner has been dropped
            if let Some(weak) = &sub.weak_owner {
                if weak.upgrade().is_none() {
                    to_remove.push(sub.id);
                    continue;
                }
            }
            (sub.handler)(&event);
            if sub.once {
                to_remove.push(sub.id);
            }
        }

        if !to_remove.is_empty() {
            self.subscribers.retain(|s| !to_remove.contains(&s.id));
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
        self.subscribers.push(Subscriber {
            id,
            handler: Box::new(handler),
            once: false,
            weak_owner: None,
        });
        id
    }

    /// Unsubscribe a handler by its id.
    pub fn unsubscribe(&mut self, id: SubscriberId) -> bool {
        let prev_len = self.subscribers.len();
        self.subscribers.retain(|s| s.id != id);
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

    /// Subscribe with a filter predicate. Handler is called only if predicate returns true.
    pub fn subscribe_with_filter<F, P>(&mut self, handler: F, predicate: P) -> SubscriberId
    where
        F: Fn(&E) + Send + Sync + 'static,
        P: Fn(&E) -> bool + Send + Sync + 'static,
    {
        self.subscribe(move |e| {
            if predicate(e) {
                handler(e);
            }
        })
    }

    /// Subscribe with a mapping function. Handler is called with mapped value if mapping returns Some.
    pub fn subscribe_with_map<F, U, M>(&mut self, handler: F, map: M) -> SubscriberId
    where
        F: Fn(U) + Send + Sync + 'static,
        M: Fn(&E) -> Option<U> + Send + Sync + 'static,
        U: 'static,
    {
        self.subscribe(move |e| {
            if let Some(mapped) = map(e) {
                handler(mapped);
            }
        })
    }

    /// Subscribe a handler that is called only once, then automatically unsubscribed.
    pub fn subscribe_once<F>(&mut self, handler: F) -> SubscriberId
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let id = self.next_subscriber_id;
        self.next_subscriber_id += 1;
        self.subscribers.push(Subscriber {
            id,
            handler: Box::new(handler),
            once: true,
            weak_owner: None,
        });
        id
    }

    /// Subscribe a handler that is automatically unsubscribed when the owner is dropped.
    /// Returns the subscriber ID.
    pub fn subscribe_weak<F>(&mut self, owner: &Arc<()>, handler: F) -> SubscriberId
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let id = self.next_subscriber_id;
        self.next_subscriber_id += 1;
        self.subscribers.push(Subscriber {
            id,
            handler: Box::new(handler),
            once: false,
            weak_owner: Some(Arc::downgrade(owner)),
        });
        id
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
