use super::World;
use crate::ecs::registry::ComponentRegistry;
use std::sync::{Arc, Mutex};

impl World {
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)
    }

    pub fn load_from_file(
        path: &std::path::Path,
        registry: Arc<Mutex<ComponentRegistry>>,
    ) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        let mut world: Self = serde_json::from_str(&json)?;
        world.registry = registry;
        Ok(world)
    }
}
