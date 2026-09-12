use super::PyWorld;
use crate::PyObject;
use engine_core::ecs::world::WeatherCondition;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Weather API
pub trait WeatherApi {
    /// Get the current weather state as a dict.
    fn get_weather(&self, py: Python) -> PyObject;
    /// Set the weather directly (scripting control).
    fn set_weather(&self, condition: String, intensity: f64, duration: u32);
    /// Get the current visibility modifier (0.0–1.0).
    fn get_weather_visibility_modifier(&self) -> f64;
}

impl WeatherApi for PyWorld {
    fn get_weather(&self, py: Python) -> PyObject {
        let world = self.inner.borrow();
        let weather = &world.weather;
        let dict = PyDict::new(py);
        dict.set_item("condition", weather.condition.as_str())
            .unwrap();
        dict.set_item("intensity", weather.intensity).unwrap();
        dict.set_item("duration_remaining", weather.duration_remaining)
            .unwrap();
        dict.into_pyobject(py).unwrap().unbind().into()
    }

    fn set_weather(&self, condition: String, intensity: f64, duration: u32) {
        let mut world = self.inner.borrow_mut();
        let old_condition = world.weather.condition;
        let new_condition = WeatherCondition::from_name(&condition);
        let new_intensity = intensity.clamp(0.0, 1.0);
        world.weather.condition = new_condition;
        world.weather.intensity = new_intensity;
        world.weather.duration_remaining = duration;
        // Emits a "weather_changed" event like natural transitions (OQ4).
        let _ = world.send_event(
            "weather_changed",
            serde_json::json!({
                "old_condition": old_condition.as_str(),
                "new_condition": new_condition.as_str(),
                "intensity": new_intensity,
            }),
        );
    }

    fn get_weather_visibility_modifier(&self) -> f64 {
        let world = self.inner.borrow();
        world.visibility_modifier
    }
}
