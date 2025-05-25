use super::PyWorld;
use pyo3::prelude::*;

pub trait EntityApi {
    fn spawn_entity(&self) -> u32;
    fn despawn_entity(&self, entity_id: u32);
    fn get_entities(&self) -> PyResult<Vec<u32>>;
    fn is_entity_alive(&self, entity_id: u32) -> bool;
}

impl EntityApi for PyWorld {
    fn spawn_entity(&self) -> u32 {
        let mut world = self.inner.borrow_mut();
        world.spawn_entity()
    }

    fn despawn_entity(&self, entity_id: u32) {
        let mut world = self.inner.borrow_mut();
        world.despawn_entity(entity_id);
        world.entities.retain(|&e| e != entity_id);
    }

    fn get_entities(&self) -> PyResult<Vec<u32>> {
        let world = self.inner.borrow_mut();
        Ok(world.get_entities())
    }

    fn is_entity_alive(&self, entity_id: u32) -> bool {
        let world = self.inner.borrow_mut();
        world.is_entity_alive(entity_id)
    }
}
