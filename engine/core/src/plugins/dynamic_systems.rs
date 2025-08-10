use crate::ecs::World;
use indexmap::IndexMap;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use topo_sort::{SortResults, TopoSort};

/// A function that can be run on a world
pub type DynSystemFn = Box<dyn Fn(Rc<RefCell<World>>, f32) + 'static>;

/// A registry of dynamic systems
#[derive(Default)]
pub struct DynamicSystemRegistry {
    systems: IndexMap<String, DynSystemFn>,
    dependencies: HashMap<String, Vec<String>>,
}

impl DynamicSystemRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            systems: IndexMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Check if a system is registered
    pub fn is_registered(&self, name: &str) -> bool {
        self.systems.contains_key(name)
    }

    /// Register a system
    pub fn register_system(&mut self, name: String, run: DynSystemFn) {
        self.systems.insert(name.clone(), run);
        self.dependencies.entry(name).or_default();
    }

    /// Register a system with dependencies
    pub fn register_system_with_deps(
        &mut self,
        name: String,
        dependencies: Vec<String>,
        run: DynSystemFn,
    ) {
        self.systems.insert(name.clone(), run);
        self.dependencies.insert(name, dependencies);
    }

    /// Update the dependencies of a system
    pub fn update_system_dependencies(
        &mut self,
        name: &str,
        dependencies: Vec<String>,
    ) -> Result<(), String> {
        if !self.systems.contains_key(name) {
            return Err(format!("System '{name}' not found"));
        }
        self.dependencies.insert(name.to_string(), dependencies);
        Ok(())
    }

    /// Run all systems
    pub fn run_all_systems(
        &self,
        world: Rc<RefCell<World>>,
        delta_time: f32,
    ) -> Result<(), String> {
        let order = self.topological_sort()?;
        for name in order {
            if let Some(system) = self.systems.get(&name) {
                (system)(Rc::clone(&world), delta_time);
            }
        }
        Ok(())
    }

    fn topological_sort(&self) -> Result<Vec<String>, String> {
        let mut sorter = TopoSort::new();

        for (name, deps) in &self.dependencies {
            let filtered_deps = deps
                .iter()
                .filter(|dep| self.systems.contains_key(*dep))
                .cloned()
                .collect::<Vec<_>>();
            sorter.insert(name.clone(), filtered_deps);
        }

        match sorter.into_vec_nodes() {
            SortResults::Full(order) => Ok(order),
            SortResults::Partial(cycle) => {
                Err(format!("Cycle detected in system dependencies: {cycle:?}"))
            }
        }
    }

    /// Unregister a system
    pub fn unregister_system(&mut self, name: &str) -> Result<(), String> {
        self.systems.swap_remove(name);
        self.dependencies.remove(name);
        Ok(())
    }

    /// Run a system
    pub fn run_system(
        &self,
        world: Rc<RefCell<World>>,
        name: &str,
        delta_time: f32,
    ) -> Result<(), String> {
        if let Some(system) = self.systems.get(name) {
            (system)(Rc::clone(&world), delta_time);
            Ok(())
        } else {
            Err(format!("System '{name}' not found"))
        }
    }

    /// List all systems
    pub fn list_systems(&self) -> Vec<String> {
        self.systems.keys().cloned().collect()
    }
}
