use crate::PyObject;
use crate::python_api::world::PyWorld;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Set the camera position (creates camera entity if not present)
///
/// Topology-aware: hex maps store `pos.Hex {q, r, z}`; square/none keep `pos.Square`.
pub fn set_camera(pyworld: &PyWorld, x: i64, y: i64, z: i64) {
    let mut world = pyworld.inner.borrow_mut();

    let is_hex = world
        .map
        .as_ref()
        .map(|m| m.topology_type() == "hex")
        .unwrap_or(false);
    let pos_json = if is_hex {
        serde_json::json!({ "pos": { "Hex": { "q": x, "r": y, "z": z } } })
    } else {
        serde_json::json!({ "pos": { "Square": { "x": x, "y": y, "z": z } } })
    };

    // Find or create the camera entity
    let camera_id = world
        .get_entities_with_component("Camera")
        .first()
        .cloned()
        .unwrap_or_else(|| {
            let id = world.spawn_entity();
            world
                .set_component(id, "Camera", serde_json::json!({ "x": x, "y": y, "z": z }))
                .unwrap();
            id
        });

    // Update Camera component with x, y, and z
    world
        .set_component(
            camera_id,
            "Camera",
            serde_json::json!({ "x": x, "y": y, "z": z }),
        )
        .unwrap();

    // Update Position component with the topology-matching variant
    world
        .set_component(camera_id, "Position", pos_json)
        .unwrap();
}

/// Get the current camera position as a Python dict {x, y, z}
///
/// Reads the Position variant matching the current topology first, then falls
/// back to the other variant. Returns None if no camera entity exists.
pub fn get_camera(pyworld: &PyWorld, py: Python) -> PyObject {
    let world = pyworld.inner.borrow();

    if let Some(camera_id) = world.get_entities_with_component("Camera").first()
        && let Some(pos) = world.get_component(*camera_id, "Position")
    {
        let is_hex = world
            .map
            .as_ref()
            .map(|m| m.topology_type() == "hex")
            .unwrap_or(false);
        let variants: [&str; 2] = if is_hex {
            ["Hex", "Square"]
        } else {
            ["Square", "Hex"]
        };
        for variant in variants {
            if let Some(cell) = pos["pos"].get(variant) {
                let (xk, yk) = if variant == "Hex" {
                    ("q", "r")
                } else {
                    ("x", "y")
                };
                let x = cell.get(xk).and_then(|v| v.as_i64()).unwrap_or(0);
                let y = cell.get(yk).and_then(|v| v.as_i64()).unwrap_or(0);
                let z = cell.get("z").and_then(|v| v.as_i64()).unwrap_or(0);
                let dict = PyDict::new(py);
                dict.set_item("x", x).unwrap();
                dict.set_item("y", y).unwrap();
                dict.set_item("z", z).unwrap();
                return dict.into();
            }
        }
    }
    py.None()
}
