use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

/// Saves the world to a temp file and loads it back, returning the loaded world.
/// Panics on error.
pub fn save_and_load_roundtrip(world: &World, registry: Arc<Mutex<ComponentRegistry>>) -> World {
    let file = NamedTempFile::new().unwrap();
    world.save_to_file(file.path()).unwrap();
    World::load_from_file(file.path(), registry).unwrap()
}
