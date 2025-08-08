use crate::python_api::world::PyWorld;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Set the camera position (creates camera entity if not present)
pub fn set_camera(pyworld: &PyWorld, x: i64, y: i64) {
    let mut world = pyworld.inner.borrow_mut();

    // Find or create the camera entity
    let camera_id = world
        .get_entities_with_component("Camera")
        .first()
        .cloned()
        .unwrap_or_else(|| {
            let id = world.spawn_entity();
            world
                .set_component(id, "Camera", serde_json::json!({ "x": x, "y": y }))
                .unwrap();
            id
        });

    // Update Camera component with x and y
    world
        .set_component(camera_id, "Camera", serde_json::json!({ "x": x, "y": y }))
        .unwrap();

    // Update Position component with {x, y, z: 0}
    world
        .set_component(
            camera_id,
            "Position",
            serde_json::json!({ "pos": { "Square": { "x": x, "y": y, "z": 0 } } }),
        )
        .unwrap();
}

/// Get the current camera position as a Python dict {x, y}
pub fn get_camera(pyworld: &PyWorld, py: Python) -> PyObject {
    let world = pyworld.inner.borrow();

    if let Some(camera_id) = world.get_entities_with_component("Camera").first()
        && let Some(pos) = world.get_component(*camera_id, "Position") {
            let x = pos["pos"]["Square"]["x"].as_i64().unwrap_or(0);
            let y = pos["pos"]["Square"]["y"].as_i64().unwrap_or(0);
            let dict = PyDict::new(py);
            dict.set_item("x", x).unwrap();
            dict.set_item("y", y).unwrap();
            return dict.into();
        }
    py.None()
}
