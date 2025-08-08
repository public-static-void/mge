use crate::python_api::world::PyWorld;
use engine_core::ecs::event::{EventBus, EventReader};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde_json::Value;
use serde_pyobject::to_pyobject;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type EventBusMap = Mutex<HashMap<String, Arc<Mutex<EventBus<Value>>>>>;

static EVENT_BUSES: once_cell::sync::Lazy<EventBusMap> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// Send an event
pub fn send_event(event_type: String, payload: String) -> PyResult<()> {
    let mut buses = EVENT_BUSES.lock().unwrap();
    let bus = buses
        .entry(event_type.clone())
        .or_insert_with(|| Arc::new(Mutex::new(EventBus::<Value>::default())))
        .clone();

    let json_payload: Value =
        serde_json::from_str(&payload).map_err(|e| PyValueError::new_err(e.to_string()))?;
    bus.lock().unwrap().send(json_payload);
    Ok(())
}

/// Poll events
pub fn poll_event(py: Python, event_type: String) -> PyResult<Vec<PyObject>> {
    let mut buses = EVENT_BUSES.lock().unwrap();
    let bus = buses
        .entry(event_type.clone())
        .or_insert_with(|| Arc::new(Mutex::new(EventBus::<Value>::default())))
        .clone();
    let mut reader = EventReader::default();
    let events: Vec<Value> = reader.read(&*bus.lock().unwrap()).cloned().collect();
    Ok(events
        .into_iter()
        .map(|e| to_pyobject(py, &e).unwrap().into())
        .collect())
}

/// Update event buses
pub fn update_event_buses() {
    let buses = EVENT_BUSES.lock().unwrap();
    for bus in buses.values() {
        bus.lock().unwrap().update();
    }
}

/// Poll ECS events
pub fn poll_ecs_event(
    pyworld: &PyWorld,
    py: Python,
    event_type: String,
) -> PyResult<Vec<PyObject>> {
    let mut world = pyworld.inner.borrow_mut();
    let events = world.take_events(&event_type);
    Ok(events
        .into_iter()
        .map(|e| serde_pyobject::to_pyobject(py, &e).unwrap().into())
        .collect())
}
