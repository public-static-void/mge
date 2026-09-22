use super::PyWorld;

/// Temperature API
pub trait TemperatureApi {
    /// Get the current global ambient temperature in °C.
    fn get_temperature(&self) -> f64;
    /// Hold ambient at `ambient` °C (clamped to [-60, 60], emits
    /// `temperature_changed`).
    fn set_temperature(&self, ambient: f64);
    /// Get the current relative humidity in `[0.0, 1.0]`.
    fn get_humidity(&self) -> f64;
    /// Set relative humidity (clamped to `[0.0, 1.0]`, non-finite ignored).
    fn set_humidity(&self, humidity: f64);
    /// Get the current atmospheric pressure in hPa.
    fn get_pressure(&self) -> f64;
    /// Set atmospheric pressure (clamped to `[900.0, 1100.0]`, non-finite ignored).
    fn set_pressure(&self, pressure: f64);
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

    fn get_humidity(&self) -> f64 {
        let world = self.inner.borrow();
        world.get_humidity()
    }

    fn set_humidity(&self, humidity: f64) {
        let mut world = self.inner.borrow_mut();
        world.set_humidity(humidity);
    }

    fn get_pressure(&self) -> f64 {
        let world = self.inner.borrow();
        world.get_pressure()
    }

    fn set_pressure(&self, pressure: f64) {
        let mut world = self.inner.borrow_mut();
        world.set_pressure(pressure);
    }
}
