use crate::ecs::world::World;
use indexmap::IndexMap;
use std::cell::RefCell;
use topo_sort::{SortResults, TopoSort};

/// A trait for systems
pub trait System: Send + Sync {
    /// Returns the name of the system
    fn name(&self) -> &'static str;
    /// Runs the system
    fn run(&mut self, world: &mut World, lua: Option<&mlua::Lua>);
    /// Returns a list of dependencies
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }
}

/// A registry of systems
pub struct SystemRegistry {
    systems: IndexMap<String, RefCell<Box<dyn System>>>,
}

impl SystemRegistry {
    /// Create a new system registry
    pub fn new() -> Self {
        Self {
            systems: IndexMap::new(),
        }
    }

    /// Register a system
    pub fn register_system<S: System + 'static>(&mut self, system: S) {
        self.systems.insert(
            system.name().to_string(),
            std::cell::RefCell::new(Box::new(system)),
        );
    }

    /// Take a system by name
    pub fn take_system(&mut self, name: &str) -> Option<std::cell::RefCell<Box<dyn System>>> {
        self.systems.shift_remove(name)
    }

    /// Register a system boxed
    pub fn register_system_boxed(
        &mut self,
        name: String,
        system: std::cell::RefCell<Box<dyn System>>,
    ) {
        self.systems.insert(name, system);
    }

    /// List all registered systems
    pub fn list_systems(&self) -> Vec<String> {
        self.systems.keys().cloned().collect()
    }

    /// Check if a system is registered
    pub fn is_registered(&self, name: &str) -> bool {
        self.systems.contains_key(name)
    }

    /// Get a system
    pub fn get_system(&self, name: &str) -> Option<std::cell::Ref<'_, Box<dyn System>>> {
        self.systems.get(name).map(|cell| cell.borrow())
    }

    /// Get sorted systems
    pub fn sorted_systems(&self) -> Vec<std::cell::Ref<'_, Box<dyn System>>> {
        let names = self.sorted_system_names();
        names
            .into_iter()
            .filter_map(|name| self.get_system(&name))
            .collect()
    }

    /// Get a system mutable
    pub fn get_system_mut(&self, name: &str) -> Option<std::cell::RefMut<'_, Box<dyn System>>> {
        self.systems.get(name).map(|cell| cell.borrow_mut())
    }

    /// Returns a topologically sorted list of system names, or panics if a cycle is detected.
    pub fn sorted_system_names(&self) -> Vec<String> {
        // Create a new topo sorter
        let mut sorter = TopoSort::new();

        // Insert all nodes and their dependencies
        for (name, cell) in &self.systems {
            let deps = cell.borrow().dependencies();
            // Only add dependencies that are registered systems
            let filtered_deps = deps
                .iter()
                .filter(|&&dep| self.systems.contains_key(dep))
                .map(|&s| s.to_string())
                .collect::<Vec<_>>();
            sorter.insert(name.clone(), filtered_deps);
        }

        // Perform the topological sort
        match sorter.into_vec_nodes() {
            SortResults::Full(order) => order,
            SortResults::Partial(cycle) => {
                panic!("Cycle detected in system dependencies: {cycle:?}");
            }
        }
    }

    /// Unregister a system by name.
    pub fn unregister_system(&mut self, name: &str) {
        self.systems.shift_remove(name);
    }
}

impl Default for SystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}
