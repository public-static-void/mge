use super::PyWorld;

/// Temperature API
pub trait TemperatureApi {
    /// Get the current global ambient temperature in °C.
    fn get_temperature(&self) -> f64;
    /// Hold ambient at `ambient` °C (clamped to [-60, 60], emits
    /// `temperature_changed`).
    fn set_temperature(&self, ambient: f64);
}

impl TemperatureApi for PyWorld {
    fn get_temperature(&self) -> f64 {
        let world = self.inner.borrow();
        world.get_temperature()
    }

    fn set_temperature(&self, ambient: f64) {
        let mut world = self.inner.borrow_mut();
        world.set_temperature(ambient);
    }
}
