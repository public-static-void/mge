use super::ecs::registry::ComponentRegistry;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameMode {
    Colony,
    Roguelike,
    Editor,
}

pub trait ModeRestrictedComponent {
    fn supported_modes() -> Vec<GameMode>;
}

#[derive(Debug)]
pub enum ModeTransitionError {
    ComponentConflict,
    InvalidTransition(String),
}

pub trait ModeTransitionHook: Send + Sync {
    fn pre_transition(&self, from: &GameMode, to: &GameMode);
    fn post_transition(&self, from: &GameMode, to: &GameMode);
}

pub struct ModeManager {
    current_mode: GameMode,
    component_registry: ComponentRegistry,
    mode_component_map: HashMap<GameMode, HashSet<TypeId>>,
    transition_hooks: Vec<Box<dyn ModeTransitionHook>>,
}

impl ModeManager {
    pub fn new(initial_mode: GameMode, component_registry: ComponentRegistry) -> Self {
        Self {
            current_mode: initial_mode,
            component_registry,
            mode_component_map: HashMap::new(),
            transition_hooks: Vec::new(),
        }
    }

    pub fn register_component_mode<T: ModeRestrictedComponent + 'static>(&mut self) {
        let type_id = TypeId::of::<T>();
        let modes = T::supported_modes();

        for mode in modes {
            self.mode_component_map
                .entry(mode)
                .or_insert_with(HashSet::new)
                .insert(type_id);
        }
    }

    pub fn transition(&mut self, new_mode: GameMode) -> Result<(), ModeTransitionError> {
        for hook in &self.transition_hooks {
            hook.pre_transition(&self.current_mode, &new_mode);
        }

        self.current_mode = new_mode.clone();

        for hook in &self.transition_hooks {
            hook.post_transition(&self.current_mode, &new_mode);
        }

        Ok(())
    }

    pub fn is_component_active<T: 'static>(&self) -> bool {
        self.mode_component_map
            .get(&self.current_mode)
            .map_or(false, |components| components.contains(&TypeId::of::<T>()))
    }

    pub fn get_active_components(&self) -> Vec<TypeId> {
        self.mode_component_map
            .get(&self.current_mode)
            .map(|components| components.iter().cloned().collect())
            .unwrap_or_default()
    }
}
