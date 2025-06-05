use super::PyWorld;
use pyo3::prelude::*;

pub trait MiscApi {
    fn process_deaths(&self);
    fn process_decay(&self);
    fn count_entities_with_type(&self, type_str: String) -> usize;
    fn save_to_file(&self, path: String) -> PyResult<()>;
    fn load_from_file(&mut self, path: String) -> PyResult<()>;
}

impl MiscApi for PyWorld {
    fn process_deaths(&self) {
        let mut world = self.inner.borrow_mut();
        world.register_system(engine_core::systems::death_decay::ProcessDeaths);
        world.run_system("ProcessDeaths", None).unwrap();
    }

    fn process_decay(&self) {
        let mut world = self.inner.borrow_mut();
        world.register_system(engine_core::systems::death_decay::ProcessDecay);
        world.run_system("ProcessDecay", None).unwrap();
    }

    fn count_entities_with_type(&self, type_str: String) -> usize {
        let world = self.inner.borrow_mut();
        world.count_entities_with_type(&type_str)
    }

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
