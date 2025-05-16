use crate::scripting::world::World;
use indexmap::IndexMap;
use std::cell::RefCell;

pub trait System: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&mut self, world: &mut World);
}

pub struct SystemRegistry {
    systems: IndexMap<String, RefCell<Box<dyn System>>>,
}

impl SystemRegistry {
    pub fn new() -> Self {
        Self {
            systems: IndexMap::new(),
        }
    }

    pub fn register_system<S: System + 'static>(&mut self, system: S) {
        self.systems.insert(
            system.name().to_string(),
            std::cell::RefCell::new(Box::new(system)),
        );
    }

    pub fn take_system(&mut self, name: &str) -> Option<std::cell::RefCell<Box<dyn System>>> {
        self.systems.shift_remove(name)
    }

    pub fn register_system_boxed(
        &mut self,
        name: String,
        system: std::cell::RefCell<Box<dyn System>>,
    ) {
        self.systems.insert(name, system);
    }

    pub fn list_systems(&self) -> Vec<String> {
        self.systems.keys().cloned().collect()
    }

    pub fn get_system_mut(&self, name: &str) -> Option<std::cell::RefMut<Box<dyn System>>> {
        self.systems.get(name).map(|cell| cell.borrow_mut())
    }
}

impl Default for SystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}
