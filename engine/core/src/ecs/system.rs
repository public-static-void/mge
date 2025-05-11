use crate::scripting::world::World;
use std::collections::HashMap;

pub trait System: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&mut self, world: &mut World);
}

pub struct SystemRegistry {
    systems: HashMap<String, Box<dyn System>>,
}

impl SystemRegistry {
    pub fn new() -> Self {
        Self {
            systems: HashMap::new(),
        }
    }

    pub fn register_system<S: System + 'static>(&mut self, system: S) {
        self.systems
            .insert(system.name().to_string(), Box::new(system));
    }

    /// Internal: Used by World to avoid borrow checker issues.
    pub fn take_system(&mut self, name: &str) -> Option<Box<dyn System>> {
        self.systems.remove(name)
    }

    /// Internal: Used by World to re-insert a system after running it.
    pub fn register_system_boxed(&mut self, name: String, system: Box<dyn System>) {
        self.systems.insert(name, system);
    }

    pub fn list_systems(&self) -> Vec<String> {
        self.systems.keys().cloned().collect()
    }
}

impl Default for SystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}
