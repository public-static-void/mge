use super::PyWorld;
use pyo3::prelude::*;

pub trait MiscApi {
    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32);
    fn damage_entity(&self, entity_id: u32, amount: f32);
    fn process_deaths(&self);
    fn process_decay(&self);
    fn count_entities_with_type(&self, type_str: String) -> usize;
    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()>;
    fn save_to_file(&self, path: String) -> PyResult<()>;
    fn load_from_file(&mut self, path: String) -> PyResult<()>;
}

impl MiscApi for PyWorld {
    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32) {
        let mut world = self.inner.borrow_mut();
        world.move_entity(entity_id, dx, dy);
    }

    fn damage_entity(&self, entity_id: u32, amount: f32) {
        let mut world = self.inner.borrow_mut();
        world.damage_entity(entity_id, amount);
    }

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

    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        world
            .modify_stockpile_resource(entity_id, &kind, delta)
            .map_err(pyo3::exceptions::PyValueError::new_err)
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
