use super::World;

impl World {
    /// Returns the current game mode as a string
    pub fn get_mode(&self) -> &str {
        &self.current_mode
    }

    /// Sets the game mode
    pub fn set_mode(&mut self, mode: &str) {
        self.current_mode = mode.to_string();
        let allowed: Vec<String> = self.registry.lock().unwrap().components_for_mode(mode);
        self.components.retain(|name, _| allowed.contains(name));
    }
}
