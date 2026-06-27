//! Dungeon generation API: `generate_dungeon(config)` method on PyWorld.

use crate::python_api::PyWorld;
use crate::PyObject;
use engine_core::systems::dungeon::{DungeonConfig, DungeonGenerator};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;

/// API for dungeon generation
pub trait DungeonApi {
    /// Generate a procedural dungeon map.
    fn generate_dungeon(&self, config: HashMap<String, PyObject>) -> PyResult<PyObject>;
}

impl DungeonApi for PyWorld {
    fn generate_dungeon(&self, config: HashMap<String, PyObject>) -> PyResult<PyObject> {
        Python::attach(|py| {
            // Build DungeonConfig from Python dict
            let mut cfg = DungeonConfig::default();

            for (key, value) in &config {
                match key.as_str() {
                    "width" => {
                        if let Ok(w) = value.extract::<f64>(py) {
                            let w = w as u32;
                            if w == 0 {
                                return Err(PyValueError::new_err("Width must be positive"));
                            }
                            cfg.width = w;
                        }
                    }
                    "height" => {
                        if let Ok(h) = value.extract::<f64>(py) {
                            let h = h as u32;
                            if h == 0 {
                                return Err(PyValueError::new_err("Height must be positive"));
                            }
                            cfg.height = h;
                        }
                    }
                    "seed" => {
                        if let Ok(s) = value.extract::<f64>(py) {
                            cfg.seed = s as u64;
                        }
                    }
                    "min_room_size" => {
                        if let Ok(s) = value.extract::<f64>(py) {
                            cfg.min_room_size = s as u32;
                        }
                    }
                    "max_room_size" => {
                        if let Ok(s) = value.extract::<f64>(py) {
                            cfg.max_room_size = s as u32;
                        }
                    }
                    "max_rooms" => {
                        if let Ok(r) = value.extract::<f64>(py) {
                            cfg.max_rooms = r as u32;
                        }
                    }
                    _ => {} // Ignore unknown keys
                }
            }

            // Generate dungeon map
            let map = DungeonGenerator::generate(&cfg)
                .map_err(|e| PyValueError::new_err(e))?;

            // Convert to worldgen JSON
            let json_value = map.to_worldgen_json();

            // Use pythonize to convert serde_json::Value to Python dict
            let py_obj = pythonize::pythonize(py, &json_value)
                .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python object: {e}")))?;
            Ok(py_obj.unbind())
        })
    }
}
