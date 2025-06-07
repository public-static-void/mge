use super::World;

impl World {
    pub fn set_mode(&mut self, mode: &str) {
        self.current_mode = mode.to_string();
        let allowed: Vec<String> = self.registry.lock().unwrap().components_for_mode(mode);
        self.components.retain(|name, _| allowed.contains(name));
    }
}
