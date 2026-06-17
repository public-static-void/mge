use crate::ecs::world::World;
use serde_json::Value as JsonValue;
use std::ops::{Deref, DerefMut};

/// Trait abstracting component operations needed by ResourceReservationSystem.
/// Both World and WasmWorld implement this, enabling the reservation algorithm
/// to work with either backend without duplication.
pub trait ResourceReservationOps {
    /// Returns all entity IDs that have the named component.
    fn get_entities_with_component(&self, name: &str) -> Vec<u32>;
    /// Returns the component value for an entity, if present, as an owned JsonValue.
    fn get_component_value(&self, entity: u32, name: &str) -> Option<JsonValue>;
    /// Sets a component value for an entity.
    fn set_component_value(
        &mut self,
        entity: u32,
        name: &str,
        value: JsonValue,
    ) -> Result<(), String>;
}

impl ResourceReservationOps for World {
    fn get_entities_with_component(&self, name: &str) -> Vec<u32> {
        World::get_entities_with_component(self, name)
    }

    fn get_component_value(&self, entity: u32, name: &str) -> Option<JsonValue> {
        World::get_component(self, entity, name).cloned()
    }

    fn set_component_value(
        &mut self,
        entity: u32,
        name: &str,
        value: JsonValue,
    ) -> Result<(), String> {
        World::set_component(self, entity, name, value)
    }
}

/// Blanket impl so that smart pointers like `RefMut<'_, World>` work with the trait.
impl<W: Deref<Target = World> + DerefMut> ResourceReservationOps for W {
    fn get_entities_with_component(&self, name: &str) -> Vec<u32> {
        let w: &World = self;
        w.get_entities_with_component(name)
    }

    fn get_component_value(&self, entity: u32, name: &str) -> Option<JsonValue> {
        let w: &World = self;
        w.get_component(entity, name).cloned()
    }

    fn set_component_value(
        &mut self,
        entity: u32,
        name: &str,
        value: JsonValue,
    ) -> Result<(), String> {
        let w: &mut World = self;
        w.set_component(entity, name, value)
    }
}
