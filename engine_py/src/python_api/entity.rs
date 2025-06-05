use super::PyWorld;
use pyo3::prelude::*;

pub trait EntityApi {
    fn spawn_entity(&self) -> u32;
    fn despawn_entity(&self, entity_id: u32);
    fn get_entities(&self) -> PyResult<Vec<u32>>;
    fn is_entity_alive(&self, entity_id: u32) -> bool;
    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32);
    fn damage_entity(&self, entity_id: u32, amount: f32);
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

    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32) {
        let mut world = self.inner.borrow_mut();
        world.move_entity(entity_id, dx, dy);
    }

    fn damage_entity(&self, entity_id: u32, amount: f32) {
        let mut world = self.inner.borrow_mut();
        world.damage_entity(entity_id, amount);
    }
}
