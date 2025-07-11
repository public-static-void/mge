use engine_core::systems::job::system::events::job_event_logger;
use once_cell::sync::Lazy;
use pyo3::Bound;
use pyo3::PyObject;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};
use std::collections::HashMap;
use std::sync::Mutex;

type JobEventSubscriptions = HashMap<String, Vec<(usize, PyObject)>>;

static JOB_EVENT_SUBSCRIPTIONS: Lazy<Mutex<JobEventSubscriptions>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static SUBSCRIPTION_ID_COUNTER: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

// --- Job Event Log Querying ---

pub fn get_job_event_log(py: Python) -> PyResult<PyObject> {
    let events = job_event_logger().all();
    let py_events = PyList::empty(py);
    for event in events {
        let dict = PyDict::new(py);
        dict.set_item("timestamp", event.timestamp)?;
        dict.set_item("event_type", &event.event_type)?;
        dict.set_item("payload", serde_pyobject::to_pyobject(py, &event.payload)?)?;
        py_events.append(dict)?;
    }
    Ok(py_events.into())
}

pub fn get_job_events_by_type(py: Python, event_type: String) -> PyResult<PyObject> {
    let events = job_event_logger().get_events_by_type(&event_type);
    let py_events = PyList::empty(py);
    for event in events {
        let dict = PyDict::new(py);
        dict.set_item("timestamp", event.timestamp)?;
        dict.set_item("event_type", &event.event_type)?;
        dict.set_item("payload", serde_pyobject::to_pyobject(py, &event.payload)?)?;
        py_events.append(dict)?;
    }
    Ok(py_events.into())
}

pub fn get_job_events_since(py: Python, timestamp: u128) -> PyResult<PyObject> {
    let events = job_event_logger().get_events_since(timestamp);
    let py_events = PyList::empty(py);
    for event in events {
        let dict = PyDict::new(py);
        dict.set_item("timestamp", event.timestamp)?;
        dict.set_item("event_type", &event.event_type)?;
        dict.set_item("payload", serde_pyobject::to_pyobject(py, &event.payload)?)?;
        py_events.append(dict)?;
    }
    Ok(py_events.into())
}

pub fn get_job_events_where<'py>(
    py: Python<'py>,
    predicate: Bound<'py, PyAny>,
) -> PyResult<PyObject> {
    let events = job_event_logger().all();
    let py_events = PyList::empty(py);
    for event in events {
        let payload_obj = serde_pyobject::to_pyobject(py, &event.payload)?;
        let should_include = predicate.call1((payload_obj.clone(),))?.extract::<bool>()?;
        if should_include {
            let dict = PyDict::new(py);
            dict.set_item("timestamp", event.timestamp)?;
            dict.set_item("event_type", &event.event_type)?;
            dict.set_item("payload", payload_obj)?;
            py_events.append(dict)?;
        }
    }
    Ok(py_events.into())
}

// --- Job Event Bus Polling and Subscription ---

pub fn poll_job_event_bus(
    py: Python,
    event_type: String,
    world: &mut engine_core::ecs::world::World,
) -> PyResult<PyObject> {
    let events = world.take_events(&event_type);
    let py_events = PyList::empty(py);
    for event in events {
        let dict = PyDict::new(py);
        dict.set_item("event_type", &event_type)?;
        dict.set_item("payload", serde_pyobject::to_pyobject(py, &event)?)?;
        py_events.append(dict)?;
    }
    Ok(py_events.into())
}

pub fn subscribe_job_event_bus(
    _py: Python,
    event_type: String,
    callback: PyObject,
) -> PyResult<usize> {
    let mut counter = SUBSCRIPTION_ID_COUNTER.lock().unwrap();
    *counter += 1;
    let id = *counter;
    let mut subs = JOB_EVENT_SUBSCRIPTIONS.lock().unwrap();
    subs.entry(event_type).or_default().push((id, callback));
    Ok(id)
}

pub fn unsubscribe_job_event_bus(event_type: String, sub_id: usize) -> PyResult<()> {
    let mut subs = JOB_EVENT_SUBSCRIPTIONS.lock().unwrap();
    if let Some(vec) = subs.get_mut(&event_type) {
        vec.retain(|(id, _)| *id != sub_id);
    }
    Ok(())
}

pub fn deliver_job_event_bus_callbacks(
    py: Python,
    world: &mut engine_core::ecs::world::World,
) -> PyResult<()> {
    let subs = JOB_EVENT_SUBSCRIPTIONS.lock().unwrap();

    let mut events_by_type: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for event_type in subs.keys() {
        let events = world.take_events(event_type);
        if !events.is_empty() {
            events_by_type.insert(event_type.clone(), events);
        }
    }

    for (event_type, callbacks) in subs.iter() {
        if let Some(events) = events_by_type.get(event_type) {
            for event in events {
                for (_id, cb) in callbacks {
                    let dict = PyDict::new(py);
                    dict.set_item("event_type", event_type)?;
                    dict.set_item("payload", serde_pyobject::to_pyobject(py, event)?)?;
                    if let Err(e) = cb.call1(py, (dict,)) {
                        e.print(py);
                    }
                }
            }
        }
    }
    Ok(())
}

// --- Job Event Log Save/Load/Replay ---

#[pyfunction]
pub fn save_job_event_log_py(path: String) -> PyResult<()> {
    Python::with_gil(|_py| {
        engine_core::systems::job::system::events::save_job_event_log(&path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to save event log: {e}"))
        })
    })
}

#[pyfunction]
pub fn load_job_event_log_py(path: String) -> PyResult<()> {
    Python::with_gil(|_py| {
        engine_core::systems::job::system::events::load_job_event_log(&path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to load event log: {e}"))
        })
    })
}

#[pyfunction]
pub fn replay_job_event_log_py(world: &crate::python_api::world::PyWorld) -> PyResult<()> {
    let mut world = world.inner.borrow_mut();
    engine_core::systems::job::system::events::replay_job_event_log(&mut world);
    Ok(())
}

pub fn clear_job_event_log_py() -> PyResult<()> {
    job_event_logger().clear();
    Ok(())
}
