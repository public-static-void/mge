use super::PyWorld;

/// Game mode API
pub trait ModeApi {
    /// Set game mode
    fn set_mode(&self, mode: String);
    /// Get current game mode
    fn get_mode(&self) -> String;
    /// Get all available game modes
    fn get_available_modes(&self) -> Vec<String>;
}

impl ModeApi for PyWorld {
    fn set_mode(&self, mode: String) {
        let mut world = self.inner.borrow_mut();
        world.set_mode(&mode);
    }

    fn get_mode(&self) -> String {
        let world = self.inner.borrow();
        world.get_mode().to_string()
    }

    fn get_available_modes(&self) -> Vec<String> {
        let world = self.inner.borrow_mut();
        world
            .registry
            .lock()
            .unwrap()
            .all_modes()
            .into_iter()
            .collect()
    }
}
