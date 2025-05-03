use std::any::TypeId;
use std::collections::{HashMap, HashSet};

/// Supported game modes for the engine.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameMode {
    Colony,
    Roguelike,
    Editor,
}

/// Trait for components that are only active in certain game modes.
pub trait ModeRestrictedComponent {
    /// Returns the modes in which this component is active.
    fn supported_modes() -> Vec<GameMode>;
}

/// Errors that can occur during mode transitions.
#[derive(Debug)]
pub enum ModeTransitionError {
    ComponentConflict,
    InvalidTransition(String),
}

/// Trait for hooks that run before and after mode transitions.
pub trait ModeTransitionHook: Send + Sync {
    fn pre_transition(&self, from: &GameMode, to: &GameMode);
    fn post_transition(&self, from: &GameMode, to: &GameMode);
}

/// Manages the current game mode and mode-specific component activation.
pub struct ModeManager {
    current_mode: GameMode,
    mode_component_map: HashMap<GameMode, HashSet<TypeId>>,
    transition_hooks: Vec<Box<dyn ModeTransitionHook>>,
}

impl ModeManager {
    /// Creates a new mode manager with the given initial mode and component registry.
    pub fn new(initial_mode: GameMode) -> Self {
        Self {
            current_mode: initial_mode,
            mode_component_map: HashMap::new(),
            transition_hooks: Vec::new(),
        }
    }

    /// Registers a component for the modes it supports.
    pub fn register_component_mode<T: ModeRestrictedComponent + 'static>(&mut self) {
        let type_id = TypeId::of::<T>();
        let modes = T::supported_modes();

        for mode in modes {
            self.mode_component_map
                .entry(mode)
                .or_default()
                .insert(type_id);
        }
    }

    /// Adds a transition hook to the manager.
    pub fn add_transition_hook(&mut self, hook: Box<dyn ModeTransitionHook>) {
        self.transition_hooks.push(hook);
    }

    /// Transitions to a new game mode, running hooks before and after.
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

    /// Checks if a component is active in the current mode.
    pub fn is_component_active<T: 'static>(&self) -> bool {
        self.mode_component_map
            .get(&self.current_mode)
            .is_some_and(|components| components.contains(&TypeId::of::<T>()))
    }

    /// Returns the TypeIds of all active components in the current mode.
    pub fn get_active_components(&self) -> Vec<TypeId> {
        self.mode_component_map
            .get(&self.current_mode)
            .map(|components| components.iter().cloned().collect())
            .unwrap_or_default()
    }
}
