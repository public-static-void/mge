use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// A single logged event, with timestamp and payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggedEvent<E> {
    pub timestamp: u128,
    pub event_type: String,
    pub payload: E,
}

impl<E> LoggedEvent<E> {
    pub fn new(event_type: &str, payload: E) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        Self {
            timestamp,
            event_type: event_type.to_string(),
            payload,
        }
    }
}

/// Thread-safe, append-only event logger.
pub struct EventLogger<E> {
    events: Arc<Mutex<Vec<LoggedEvent<E>>>>,
}

impl<E: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static> EventLogger<E> {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn log(&self, event_type: &str, payload: E) {
        let event = LoggedEvent::new(event_type, payload);
        self.events.lock().unwrap().push(event);
    }

    pub fn all(&self) -> Vec<LoggedEvent<E>> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &*self.events.lock().unwrap())?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let events: Vec<LoggedEvent<E>> = serde_json::from_reader(reader)?;
        Ok(Self {
            events: Arc::new(Mutex::new(events)),
        })
    }

    /// Replay all events into the provided event bus, in order.
    pub fn replay_into<F>(&self, mut f: F)
    where
        F: FnMut(&LoggedEvent<E>),
    {
        for event in self.events.lock().unwrap().iter() {
            f(event);
        }
    }
}

impl<E: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static> Default
    for EventLogger<E>
{
    fn default() -> Self {
        Self::new()
    }
}
