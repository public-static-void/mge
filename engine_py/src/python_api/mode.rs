use super::PyWorld;

pub trait ModeApi {
    fn set_mode(&self, mode: String);
    fn get_mode(&self) -> String;
    fn get_available_modes(&self) -> Vec<String>;
}

impl ModeApi for PyWorld {
    fn set_mode(&self, mode: String) {
        let mut world = self.inner.borrow_mut();
        world.current_mode = mode;
    }

    fn get_mode(&self) -> String {
        let world = self.inner.borrow_mut();
        world.current_mode.clone()
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
