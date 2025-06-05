use super::PyWorld;
use pyo3::prelude::*;

pub trait SaveLoadApi {
    fn save_to_file(&self, path: String) -> PyResult<()>;
    fn load_from_file(&mut self, path: String) -> PyResult<()>;
}

impl SaveLoadApi for PyWorld {
    fn save_to_file(&self, path: String) -> PyResult<()> {
        let world = self.inner.borrow_mut();
        world
            .save_to_file(std::path::Path::new(&path))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    fn load_from_file(&mut self, path: String) -> PyResult<()> {
        let registry = {
            let world = self.inner.borrow_mut();
            world.registry.clone()
        };
        let loaded =
            engine_core::ecs::world::World::load_from_file(std::path::Path::new(&path), registry)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        let mut world = self.inner.borrow_mut();
        *world = loaded;
        Ok(())
    }
}
