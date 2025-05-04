//! ECS (Entity Component System) core module.
//!
//! Exposes core ECS types, schema support, and error handling.

pub mod components;
mod error;
pub(crate) mod registry;

use crate::modes::{GameMode, ModeManager, ModeRestrictedComponent};
pub use components::{Health, Position};
pub use error::{MigrationError, RegistryError};
pub use registry::{Component, ComponentRegistry};
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Represents the JSON schema for a component, used for dynamic registration and validation.
#[derive(Debug)]
pub struct ComponentSchema {
    pub name: String,
    pub schema: Option<schemars::schema::RootSchema>,
}

pub struct EcsWorld {
    entities: Vec<u32>,
    next_entity_id: u32,
    components: HashMap<TypeId, HashMap<u32, Box<dyn Any>>>,
    mode_manager: ModeManager,
}

#[derive(Debug)]
pub enum Error {
    ComponentUnavailableInMode,
    EntityNotFound,
    // Add other errors as needed
}

impl EcsWorld {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            next_entity_id: 1,
            components: HashMap::new(),
            mode_manager: ModeManager::new(GameMode::Colony), // default mode
        }
    }

    pub fn spawn(&mut self) -> u32 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.push(id);
        id
    }

    pub fn register_component<T: ModeRestrictedComponent + 'static>(&mut self) {
        self.mode_manager.register_component_mode::<T>();
    }

    pub fn set_mode(&mut self, mode: GameMode) {
        self.mode_manager
            .transition(mode)
            .expect("Invalid mode transition");
    }

    pub fn set_component<T: ModeRestrictedComponent + 'static>(
        &mut self,
        entity: u32,
        component: T,
    ) -> Result<(), Error> {
        if !self.entities.contains(&entity) {
            return Err(Error::EntityNotFound);
        }
        if !self.mode_manager.is_component_active::<T>() {
            return Err(Error::ComponentUnavailableInMode);
        }
        self.components
            .entry(TypeId::of::<T>())
            .or_default()
            .insert(entity, Box::new(component));
        Ok(())
    }

    pub fn get_component<T: ModeRestrictedComponent + 'static>(&self, entity: u32) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|map| map.get(&entity))
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }
}

impl Default for EcsWorld {
    fn default() -> Self {
        Self::new()
    }
}
