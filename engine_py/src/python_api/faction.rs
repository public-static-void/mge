use super::PyWorld;

/// API for the faction and reputation system
pub trait FactionApi {
    /// Assign an entity to a faction
    fn set_faction(&self, entity: u32, faction_id: &str, role: &str);
    /// Get the entity's faction_id, or None if not assigned to a faction
    fn get_faction(&self, entity: u32) -> Option<String>;
    /// Adjust reputation with a faction by delta
    fn modify_reputation(&self, entity: u32, faction_id: &str, delta: i64);
    /// Get the reputation score with a faction, or 0 if absent
    fn get_reputation(&self, entity: u32, faction_id: &str) -> i64;
}

impl FactionApi for PyWorld {
    fn set_faction(&self, entity: u32, faction_id: &str, role: &str) {
        let mut world = self.inner.borrow_mut();
        engine_core::faction::set_faction(&mut world, entity, faction_id, role).unwrap();
    }

    fn get_faction(&self, entity: u32) -> Option<String> {
        let world = self.inner.borrow();
        engine_core::faction::get_faction(&world, entity)
    }

    fn modify_reputation(&self, entity: u32, faction_id: &str, delta: i64) {
        let mut world = self.inner.borrow_mut();
        engine_core::faction::modify_reputation(&mut world, entity, faction_id, delta).unwrap();
    }

    fn get_reputation(&self, entity: u32, faction_id: &str) -> i64 {
        let world = self.inner.borrow();
        engine_core::faction::get_reputation(&world, entity, faction_id)
    }
}
