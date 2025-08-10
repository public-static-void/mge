use super::PyWorld;
use pyo3::prelude::*;

/// Entity API
pub trait EntityApi {
    /// Spawn a new entity
    fn spawn_entity(&self) -> u32;
    /// Despawn an entity
    fn despawn_entity(&self, entity_id: u32);
    /// Get all entities
    fn get_entities(&self) -> PyResult<Vec<u32>>;
    /// Count entities with a given type
    fn count_entities_with_type(&self, type_str: String) -> usize;
    /// Check if an entity is alive
    fn is_entity_alive(&self, entity_id: u32) -> bool;
    /// Move an entity
    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32);
    /// Damage an entity
    fn damage_entity(&self, entity_id: u32, amount: f32);
}

impl EntityApi for PyWorld {
    /// Spawn a new entity
    fn spawn_entity(&self) -> u32 {
        let mut world = self.inner.borrow_mut();
        world.spawn_entity()
    }

    /// Despawn an entity
    fn despawn_entity(&self, entity_id: u32) {
        let mut world = self.inner.borrow_mut();
        world.despawn_entity(entity_id);
        world.entities.retain(|&e| e != entity_id);
    }

    /// Get all entities
    fn get_entities(&self) -> PyResult<Vec<u32>> {
        let world = self.inner.borrow_mut();
        Ok(world.get_entities())
    }

    /// Count entities with a given type
    fn count_entities_with_type(&self, type_str: String) -> usize {
        let world = self.inner.borrow_mut();
        world.count_entities_with_type(&type_str)
    }

    /// Check if an entity is alive
    fn is_entity_alive(&self, entity_id: u32) -> bool {
        let world = self.inner.borrow_mut();
        world.is_entity_alive(entity_id)
    }

    /// Move an entity
    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32) {
        let mut world = self.inner.borrow_mut();
        world.move_entity(entity_id, dx, dy);
    }

    /// Damage an entity
    fn damage_entity(&self, entity_id: u32, amount: f32) {
        let mut world = self.inner.borrow_mut();
        world.damage_entity(entity_id, amount);
    }
}
